use std::future::Future;
use std::task::Poll;

use sha2::{Digest, Sha512};

use super::*;
use crate::DownloadComponent;

fn estimated_download(
    fixture: &Fixture,
    url: &str,
    size: u64,
) -> (
    tokio::task::JoinHandle<Result<()>>,
    tokio::sync::mpsc::Receiver<crate::DownloadProgress>,
) {
    let task = crate::DownloadTask {
        url: url.to_string(),
        destination: fixture.game.root().join("payload.bin"),
        sha1: SHA1.to_string(),
        size,
    };
    let (sender, incoming) = tokio::sync::mpsc::channel(64);
    let context = InstallationContext::from(sender).with_size_hints();
    let running = tokio::spawn(async move {
        context
            .queue(
                &reqwest::Client::new(),
                &[task],
                crate::DownloadOptions {
                    retry: crate::Retry {
                        attempts: 1,
                        initial_backoff: Duration::from_millis(1),
                    },
                    ..crate::DownloadOptions::default()
                },
            )
            .await
    });
    (running, incoming)
}

async fn final_estimated_progress(
    mut incoming: tokio::sync::mpsc::Receiver<crate::DownloadProgress>,
) -> crate::DownloadProgress {
    let mut last = None;
    while let Some(progress) = incoming.recv().await {
        assert!(
            progress.bytes_done <= progress.bytes_total,
            "estimated progress must not exceed its total: {progress:?}"
        );
        last = Some(progress);
    }
    last.expect("progress")
}

#[tokio::test]
async fn modpack_size_hints_accept_verified_files_and_correct_progress_in_both_directions() {
    for size in [10, 12] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let (running, incoming) = estimated_download(&fixture, &server.url, size);
        server.request().await.reply(200, "", BODY, BODY.len());
        finish(async { running.await.expect("worker") })
            .await
            .expect("verified file despite stale size");
        assert_eq!(
            std::fs::read(fixture.game.root().join("payload.bin")).expect("file"),
            BODY
        );
        let progress = final_estimated_progress(incoming).await;
        assert_eq!(
            (
                progress.finished,
                progress.failed,
                progress.bytes_done,
                progress.bytes_total
            ),
            (1, 0, 11, 11)
        );
        fixture.close().await;
    }
}

#[tokio::test]
async fn modpack_size_hints_preserve_and_resume_partials_larger_than_the_estimate() {
    for size in [4, 12] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        std::fs::create_dir_all(fixture.game.root()).expect("directory");
        std::fs::write(fixture.part(), b"hello").expect("partial");
        let (running, incoming) = estimated_download(&fixture, &server.url, size);
        let request = server.request().await;
        assert!(
            request
                .headers
                .to_ascii_lowercase()
                .contains("range: bytes=5-\r\n")
        );
        request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
        finish(async { running.await.expect("worker") })
            .await
            .expect("resumed file");
        assert_eq!(
            std::fs::read(fixture.game.root().join("payload.bin")).expect("file"),
            BODY
        );
        let progress = final_estimated_progress(incoming).await;
        assert_eq!((progress.bytes_done, progress.bytes_total), (11, 11));
        assert!(!fixture.part().exists());
        fixture.close().await;
    }
}

#[tokio::test]
async fn modpack_size_hints_still_reject_corrupt_content() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let (running, incoming) = estimated_download(&fixture, &server.url, 12);
    server.request().await.reply(200, "", b"wrong bytes", 11);
    let result = finish(async { running.await.expect("worker") }).await;
    assert!(matches!(result, Err(CoreError::Sha1Mismatch { .. })));
    assert!(!fixture.game.root().join("payload.bin").exists());
    assert!(!fixture.part().exists());
    let progress = final_estimated_progress(incoming).await;
    assert_eq!(
        (progress.finished, progress.failed, progress.bytes_done),
        (0, 1, 0)
    );
    fixture.close().await;
}

#[tokio::test]
async fn modpack_size_hints_still_reject_invalid_ranges() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    std::fs::create_dir_all(fixture.game.root()).expect("directory");
    std::fs::write(fixture.part(), b"hello").expect("partial");
    let (running, _) = estimated_download(&fixture, &server.url, 12);
    server
        .request()
        .await
        .reply(206, "Content-Range: bytes 4-10/11\r\n", b" world", 6);
    assert!(matches!(
        finish(async { running.await.expect("worker") }).await,
        Err(CoreError::DownloadResponse { .. })
    ));
    assert!(!fixture.game.root().join("payload.bin").exists());
    assert!(!fixture.part().exists());
    fixture.close().await;
}

#[tokio::test]
async fn modpack_size_hints_use_actual_size_for_verified_cached_files() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    std::fs::create_dir_all(fixture.game.root()).expect("directory");
    std::fs::write(fixture.game.root().join("payload.bin"), BODY).expect("cached file");
    let (running, incoming) = estimated_download(&fixture, &server.url, 12);
    finish(async { running.await.expect("worker") })
        .await
        .expect("cached file");
    let progress = final_estimated_progress(incoming).await;
    assert_eq!((progress.bytes_done, progress.bytes_total), (11, 11));
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn modpack_size_hints_publish_complete_verified_partials_without_a_request() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    std::fs::create_dir_all(fixture.game.root()).expect("directory");
    std::fs::write(fixture.part(), BODY).expect("complete partial");
    let (mut running, incoming) = estimated_download(&fixture, &server.url, 4);
    finish(async {
        tokio::select! {
            result = &mut running => result.expect("worker").expect("verified partial"),
            _ = server.request() => panic!("a verified complete partial must not be downloaded again"),
        }
    }).await;
    let progress = final_estimated_progress(incoming).await;
    assert_eq!((progress.bytes_done, progress.bytes_total), (11, 11));
    assert_eq!(
        std::fs::read(fixture.game.root().join("payload.bin")).expect("file"),
        BODY
    );
    fixture.close().await;
}

#[tokio::test]
async fn modpack_size_hints_support_chunked_bodies_without_content_length() {
    for size in [4, 12] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let (running, incoming) = estimated_download(&fixture, &server.url, size);
        server.request().await.reply_chunked(BODY);
        finish(async { running.await.expect("worker") })
            .await
            .expect("verified chunked response");
        let progress = final_estimated_progress(incoming).await;
        assert_eq!((progress.bytes_done, progress.bytes_total), (11, 11));
        assert_eq!(
            std::fs::read(fixture.game.root().join("payload.bin")).expect("file"),
            BODY
        );
        fixture.close().await;
    }
}

#[tokio::test]
async fn modpack_size_hints_require_a_published_checksum_before_any_request() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let task = crate::VerifiedDownloadTask {
        url: server.url.clone(),
        destination: fixture.game.root().join("payload.bin"),
        checksum: crate::DownloadChecksum::Unverified,
        size: 12,
    };
    let mut running = tokio::spawn(async move {
        InstallationContext::default()
            .with_size_hints()
            .verified_queue(
                &reqwest::Client::new(),
                &[task],
                crate::DownloadOptions::default(),
            )
            .await
    });
    let result = finish(async {
        tokio::select! {
            result = &mut running => result.expect("worker"),
            _ = server.request() => panic!("size hints must not download content without a checksum"),
        }
    }).await;
    assert!(matches!(
        result,
        Err(CoreError::DownloadResponse {
            detail: "estimated download sizes require a checksum"
        })
    ));
    assert!(!fixture.game.root().join("payload.bin").exists());
    fixture.close().await;
}

#[tokio::test]
async fn modpack_size_hints_enforce_sha512_checksums() {
    for valid in [true, false] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let hash = if valid {
            crate::hashes::hex_encode(Sha512::digest(BODY).as_slice())
        } else {
            "a".repeat(128)
        };
        let task = crate::VerifiedDownloadTask {
            url: server.url.clone(),
            destination: fixture.game.root().join("payload.bin"),
            checksum: crate::DownloadChecksum::Sha512(hash),
            size: 12,
        };
        let running = tokio::spawn(async move {
            InstallationContext::default()
                .with_size_hints()
                .verified_queue(
                    &reqwest::Client::new(),
                    &[task],
                    crate::DownloadOptions {
                        retry: crate::Retry {
                            attempts: 1,
                            initial_backoff: Duration::from_millis(1),
                        },
                        ..crate::DownloadOptions::default()
                    },
                )
                .await
        });
        server.request().await.reply(200, "", BODY, BODY.len());
        let result = finish(async { running.await.expect("worker") }).await;
        if valid {
            result.expect("matching SHA512");
        } else {
            assert!(matches!(result, Err(CoreError::Sha512Mismatch { .. })));
        }
        assert_eq!(fixture.game.root().join("payload.bin").exists(), valid);
        assert!(!fixture.part().exists());
        fixture.close().await;
    }
}

fn verifying_executor(
    _runtime: DownloadRuntime,
    _job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let context = context.component(DownloadComponent::Modpack);
        context.stage(DownloadPhase::Downloading).await?;
        context.stage(DownloadPhase::Verifying).await?;
        context.metadata(std::future::pending::<Result<()>>()).await
    })
}

#[tokio::test]
async fn modpack_late_pause_returns_the_current_verifying_snapshot() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Late pause").await;
    let manager = fixture.manager_with(verifying_executor, reqwest::Client::new());
    let job = manager
        .start(&instance.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    finish(async {
        loop {
            let snapshot = fixture
                .database()
                .get_download_job(&fixture.game, &job.id)
                .await
                .expect("job");
            if snapshot.phase == DownloadPhase::Verifying {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;

    let result = manager.pause(&job.id).await;
    manager.cancel(&job.id).await.expect("stop fixture");
    fixture.close().await;
    let snapshot =
        result.expect("a late pause must return the new phase rather than a transition error");
    assert_eq!(snapshot.status, DownloadJobStatus::Running);
    assert_eq!(snapshot.phase, DownloadPhase::Verifying);
}

#[tokio::test]
async fn modpack_cached_verification_yields_and_observes_cancellation() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Cached verification").await;
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("directory");
    let mods = directory.join("mods");
    std::fs::create_dir_all(&mods).expect("mods directory");
    let body = vec![b'x'; 2 * 1024 * 1024];
    let file = mods.join("cached.jar");
    std::fs::write(&file, &body).expect("cached mod");
    let index = serde_json::json!({ "dependencies": {"minecraft": "1.20.1"}, "files": [{
        "path": "mods/cached.jar", "fileSize": body.len(),
        "hashes": { "sha1": crate::hashes::sha1_hex(&body), "sha512": crate::hashes::hex_encode(Sha512::digest(&body).as_slice()) },
        "downloads": ["https://cdn.modrinth.com/data/fixture/cached.jar"]
    }] });
    let stored =
        crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory).expect("index");
    let database = fixture.database();
    database
        .replace_modpack(
            &fixture.game.instances_dir().expect("instances"),
            &instance.folder,
            stored,
        )
        .await
        .expect("store index");
    let control = DownloadControl::new();
    let (events, mut incoming) = tokio::sync::mpsc::channel(8);
    let context = InstallationContext::for_job(
        fixture.settings.clone(),
        fixture.game.clone(),
        "cached-verification".to_string(),
        control.clone(),
        events,
        database.clone(),
    );
    let mut installation = Box::pin(crate::install_mrpack_files(
        &database,
        &fixture.game,
        &fixture.settings,
        &directory,
        context,
    ));
    // Finish the SQLite reads before checking that cached hashing yields.
    tokio::select! {
        result = &mut installation => panic!("installation ended before verification: {result:?}"),
        event = incoming.recv() => assert!(matches!(event, Some(crate::transfer::installation::InstallationEvent::Stage(DownloadComponent::Modpack, DownloadPhase::Verifying)))),
    }
    std::future::poll_fn(|cx| {
        assert!(installation.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    std::future::poll_fn(|cx| {
        assert!(
            installation.as_mut().poll(cx).is_pending(),
            "cached verification must yield instead of monopolizing the installer poll"
        );
        Poll::Ready(())
    })
    .await;
    control.cancel();
    let result = finish(installation).await;
    assert!(matches!(result, Err(CoreError::DownloadCancelled)));
    assert_eq!(std::fs::read(&file).expect("cached mod preserved"), body);
    fixture.close().await;
}

fn write_workspace(
    runtime: &DownloadRuntime,
    job: &DownloadJobSnapshot,
) -> Result<std::path::PathBuf> {
    let directory = runtime
        .game
        .instance_dir(job.instance_folder.as_deref().expect("instance"))?;
    let archive = crate::modpacks::workspace::archive_path(&directory)?;
    std::fs::create_dir_all(archive.parent().expect("workspace"))?;
    std::fs::write(&archive, b"archive")?;
    std::fs::write(archive.with_file_name("pack.zip.part"), b"partial")?;
    std::fs::write(directory.join("save.dat"), b"world")?;
    Ok(archive)
}

#[tokio::test]
async fn modpack_repair_uses_sqlite_even_when_the_legacy_backup_is_invalid() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Database repair").await;
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("directory");
    let body = b"cached mod fixture";
    std::fs::create_dir_all(directory.join("mods")).expect("mods");
    std::fs::write(directory.join("mods/cached.jar"), body).expect("file");
    let index = serde_json::json!({ "dependencies":{"minecraft":"1.20.1"}, "files":[{
        "path":"mods/cached.jar", "fileSize":body.len(),
        "hashes":{"sha1":crate::hashes::sha1_hex(body), "sha512":crate::hashes::hex_encode(Sha512::digest(body).as_slice())},
        "downloads":["https://cdn.modrinth.com/data/fixture/cached.jar"]
    }]});
    let database = fixture.database();
    let stored =
        crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory).expect("index");
    database
        .replace_modpack(
            &fixture.game.instances_dir().expect("instances"),
            &instance.folder,
            stored,
        )
        .await
        .expect("index");
    std::fs::write(
        directory.join(crate::MODPACK_INDEX_FILE),
        b"invalid preserved backup",
    )
    .expect("backup");
    for _ in 0..2 {
        crate::install_mrpack_files(
            &database,
            &fixture.game,
            &fixture.settings,
            &directory,
            InstallationContext::default(),
        )
        .await
        .expect("verify cached files without network");
    }
    assert_eq!(
        std::fs::read(directory.join("mods/cached.jar")).expect("file"),
        body
    );
    assert_eq!(
        std::fs::read(directory.join(crate::MODPACK_INDEX_FILE)).expect("backup"),
        b"invalid preserved backup"
    );
    fixture.close().await;
}

#[tokio::test]
async fn modpack_cancellation_removes_new_indexes_but_preserves_repair_indexes() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let manager = fixture.manager_with(verifying_executor, reqwest::Client::new());
    let instances = fixture.game.instances_dir().expect("instances");
    for (name, operation) in [
        ("New pack", DownloadJobOperation::Install),
        ("Existing pack", DownloadJobOperation::Repair),
    ] {
        let instance = fixture.instance(name).await;
        let directory = instances.join(&instance.folder);
        let index = serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[]});
        let stored = crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory)
            .expect("index");
        fixture
            .database()
            .replace_modpack(&instances, &instance.folder, stored)
            .await
            .expect("store");
        let job = manager
            .start(&instance.folder, operation)
            .await
            .expect("start");
        finish(async {
            loop {
                if fixture
                    .database()
                    .get_download_job(&fixture.game, &job.id)
                    .await
                    .expect("job")
                    .phase
                    == DownloadPhase::Verifying
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
        manager.cancel(&job.id).await.expect("cancel");
        let preserved = operation == DownloadJobOperation::Repair;
        assert_eq!(directory.exists(), preserved);
        assert_eq!(
            fixture
                .database()
                .modpack_header(&instances, &instance.folder)
                .await
                .expect("index")
                .is_some(),
            preserved
        );
    }
    fixture.close().await;
}

fn workspace_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        write_workspace(&runtime, &job)?;
        context.stage(DownloadPhase::Downloading).await?;
        if job.operation == DownloadJobOperation::Repair {
            return Err(std::io::Error::other("fixture failure").into());
        }
        Ok(())
    })
}

fn paused_workspace_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        write_workspace(&runtime, &job)?;
        context.stage(DownloadPhase::Downloading).await?;
        context.metadata(std::future::pending::<Result<()>>()).await
    })
}

#[tokio::test]
async fn modpack_workspace_is_removed_on_success_and_preserved_on_failure_until_cancelled() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let manager = fixture.manager_with(workspace_executor, reqwest::Client::new());
    let success = fixture.instance("Complete workspace").await;
    let job = manager
        .start(&success.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    assert_eq!(
        finish(manager.wait(&job.id)).await.expect("finish").status,
        DownloadJobStatus::Completed
    );
    assert!(
        !fixture
            .game
            .instance_dir(&success.folder)
            .expect("instance")
            .join(".install")
            .exists()
    );
    let failed = fixture.instance("Failed workspace").await;
    let job = manager
        .start(&failed.folder, DownloadJobOperation::Repair)
        .await
        .expect("repair");
    assert_eq!(
        finish(manager.wait(&job.id))
            .await
            .expect("failed snapshot")
            .status,
        DownloadJobStatus::Failed
    );
    let directory = fixture.game.instance_dir(&failed.folder).expect("instance");
    assert!(directory.join(".install").join("pack.zip").is_file());
    assert_eq!(
        manager.cancel(&job.id).await.expect("cancel").status,
        DownloadJobStatus::Cancelled
    );
    assert!(!directory.join(".install").exists());
    assert_eq!(
        std::fs::read(directory.join("save.dat")).expect("preserved save"),
        b"world"
    );
    fixture.close().await;
}

#[tokio::test]
async fn modpack_workspace_survives_pause_and_resume_and_is_removed_after_cancellation() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Paused workspace").await;
    let manager = fixture.manager_with(paused_workspace_executor, reqwest::Client::new());
    let job = manager
        .start(&instance.folder, DownloadJobOperation::Repair)
        .await
        .expect("repair");
    finish(async {
        loop {
            if fixture
                .database()
                .get_download_job(&fixture.game, &job.id)
                .await
                .expect("job")
                .phase
                == DownloadPhase::Downloading
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("instance");
    assert_eq!(
        manager.pause(&job.id).await.expect("pause").status,
        DownloadJobStatus::Paused
    );
    assert!(directory.join(".install").join("pack.zip.part").is_file());
    assert_eq!(
        manager.resume(&job.id).await.expect("resume").status,
        DownloadJobStatus::Running
    );
    assert!(directory.join(".install").join("pack.zip").is_file());
    assert_eq!(
        manager.cancel(&job.id).await.expect("cancel").status,
        DownloadJobStatus::Cancelled
    );
    assert!(!directory.join(".install").exists());
    fixture.close().await;
}
