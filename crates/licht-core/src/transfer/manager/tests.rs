use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use super::*;
mod pipeline;
use crate::{
    Arch, DownloadChecksum, DownloadComponent, InstancePreparationState, OsName,
    VerifiedDownloadTask,
};

const BODY: &[u8] = b"hello world";
const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

fn fixture_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let connection = crate::open_db(&runtime.settings)?;
        let url =
            crate::get_meta(&connection, "test_download_url")?.ok_or(CoreError::VersionMissing)?;
        let finish_after_cancel =
            crate::get_meta(&connection, "test_finish_after_cancel")?.is_some();
        drop(connection);
        let component = if job.java_major.is_some() {
            DownloadComponent::Java
        } else {
            DownloadComponent::Minecraft
        };
        let context = context.component(component);
        let task = VerifiedDownloadTask {
            url,
            destination: runtime.game.root().join("payload.bin"),
            checksum: DownloadChecksum::Sha1(SHA1.into()),
            size: 11,
        };
        context
            .verified_queue(&runtime.client, &[task], context.options()?)
            .await?;
        context.checkpoint().await?;
        if let Some(folder) = &job.instance_folder {
            crate::configure_instance_version(&runtime.game.instances_dir()?, folder, "1.21.1")?;
            crate::complete_instance_preparation(&runtime.game.instances_dir()?, folder)?;
        }
        if let Some(major) = job.java_major {
            let root = runtime
                .game
                .root()
                .join("runtime")
                .join(format!("temurin{major}-jre{major}.0.1-linux_x64"));
            std::fs::create_dir_all(root.join("bin"))?;
            let executable = root.join("bin").join("java");
            std::fs::write(&executable, b"test executable")?;
            std::fs::write(
                root.join("release"),
                format!("JAVA_VERSION=\"{major}.0.1\""),
            )?;
            crate::set_java_path(&runtime.settings, major, &executable)?;
        }
        if finish_after_cancel {
            context.control.cancelled().await;
        }
        Ok(())
    })
}

#[tokio::test]
async fn pause_resume_and_completion_use_the_persisted_job_and_block_early_launch() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Survival");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = tokio::time::timeout(Duration::from_secs(10), server.requests.recv())
        .await
        .expect("initial download request timeout")
        .expect("initial request")
        .hold();
    fixture.partial(5).await;
    let paused = tokio::time::timeout(Duration::from_secs(10), manager.pause(&job.id))
        .await
        .expect("pause did not acknowledge idle transfers")
        .expect("pause");
    assert_eq!(paused.status, DownloadJobStatus::Paused);
    assert!(matches!(
        manager.ensure_launchable(&entry.folder),
        Err(CoreError::InstancePreparing)
    ));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if get_download_job(&fixture.settings, &fixture.game, &job.id)
                .is_ok_and(|job| job.progress.bytes_done == 5)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("paused byte progress was not persisted");
    assert_eq!(std::fs::read(fixture.part()).expect("partial"), b"hello");
    assert!(server.requests.is_empty());
    let resumed = tokio::time::timeout(Duration::from_secs(10), manager.resume(&job.id))
        .await
        .expect("resume did not acknowledge idle transfers")
        .expect("resume");
    assert_eq!(resumed.status, DownloadJobStatus::Running, "{resumed:?}");
    assert_eq!(
        manager
            .active_job(&job.id)
            .await
            .expect("active job")
            .control
            .state(),
        DownloadState::Running
    );
    let request = tokio::time::timeout(Duration::from_secs(10), async { tokio::select! {
        request = server.requests.recv() => request.expect("resume request"),
        outcome = manager.wait(&job.id) => panic!("job ended before issuing its resume request: {outcome:?}"),
    }}).await.expect("resumed transfer did not issue its Range request");
    assert!(
        request
            .headers
            .to_ascii_lowercase()
            .contains("range: bytes=5-")
    );
    request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
    let completed = tokio::time::timeout(Duration::from_secs(10), manager.wait(&job.id))
        .await
        .expect("installation did not finish after the resumed response")
        .expect("complete");
    assert_eq!(completed.status, DownloadJobStatus::Completed);
    manager
        .ensure_launchable(&entry.folder)
        .expect("launch enabled after preparation");
    assert!(
        list_download_jobs(&fixture.settings, &fixture.game)
            .expect("list")
            .is_empty()
    );
    assert_eq!(
        std::fs::read(fixture.game.root().join("payload.bin")).expect("file"),
        BODY
    );
    assert_eq!(
        manager.cancel(&job.id).await.expect("completed job").status,
        DownloadJobStatus::Completed
    );
    assert!(
        fixture
            .game
            .instance_dir(&entry.folder)
            .expect("folder")
            .exists()
    );
    drop(tail);
}

#[tokio::test]
async fn shutdown_preserves_partial_bytes_and_a_new_manager_resumes_the_same_job() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Restart");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    finish(manager.suspend_all()).await.expect("shutdown");
    assert!(manager.active.lock().await.is_empty());
    assert_eq!(
        get_download_job(&fixture.settings, &fixture.game, &job.id)
            .expect("stored")
            .status,
        DownloadJobStatus::Interrupted
    );
    assert_eq!(
        std::fs::read(fixture.part()).expect("partial survives shutdown"),
        b"hello"
    );
    assert!(manager.resume(&job.id).await.is_err());
    drop(manager);
    drop(tail);

    crate::recover_download_jobs(&fixture.settings, &fixture.game).expect("startup");
    let restarted = fixture.manager();
    assert!(
        server.requests.is_empty(),
        "startup must not resume automatically"
    );
    restarted.resume(&job.id).await.expect("manual resume");
    let request = server.request().await;
    assert!(
        request
            .headers
            .to_ascii_lowercase()
            .contains("range: bytes=5-")
    );
    request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
    let result = finish(restarted.wait(&job.id)).await.expect("completion");
    assert_eq!(result.id, job.id);
    assert_eq!(result.status, DownloadJobStatus::Completed);
    assert!(!fixture.part().exists());
}

#[tokio::test]
async fn cancelling_a_new_installation_removes_the_instance_after_the_worker_stops() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Cancel");
    let manager = fixture.manager();
    let worker = Arc::clone(&manager);
    let folder = entry.folder.clone();
    let preparation =
        tokio::spawn(async move { worker.prepare(&folder, DownloadJobOperation::Install).await });
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    let job = list_download_jobs(&fixture.settings, &fixture.game)
        .expect("list")
        .pop()
        .expect("job");
    let cancelled = finish(manager.cancel(&job.id)).await.expect("cancel");
    assert_eq!(cancelled.status, DownloadJobStatus::Cancelled);
    assert_eq!(
        finish(async { preparation.await.expect("join") })
            .await
            .expect("outcome")
            .status,
        DownloadJobStatus::Cancelled
    );
    assert!(!fixture.part().exists());
    assert!(
        !fixture
            .game
            .instance_dir(&entry.folder)
            .expect("folder")
            .exists()
    );
    assert!(
        crate::list_instances(&fixture.game.instances_dir().expect("root"))
            .expect("library")
            .is_empty()
    );
    assert_eq!(
        manager
            .cancel(&job.id)
            .await
            .expect("already cancelled")
            .status,
        DownloadJobStatus::Cancelled
    );
    drop(tail);
}

#[tokio::test]
async fn cancelling_repairs_and_updates_preserves_existing_instances_and_saves() {
    for operation in [DownloadJobOperation::Repair, DownloadJobOperation::Install] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url);
        let entry = fixture.instance("Existing");
        let root = fixture.game.instances_dir().expect("root");
        crate::configure_instance_version(&root, &entry.folder, "1.20.1").expect("version");
        crate::complete_instance_preparation(&root, &entry.folder).expect("already installed");
        let dir = fixture.game.instance_dir(&entry.folder).expect("folder");
        std::fs::create_dir_all(dir.join("saves")).expect("saves");
        let save = dir.join("saves/world.dat");
        std::fs::write(&save, b"existing world").expect("save");
        let manager = fixture.manager();
        let job = manager
            .start(&entry.folder, operation)
            .await
            .expect("start");
        let tail = server.request().await.hold();
        fixture.partial(5).await;
        assert_eq!(
            finish(manager.cancel(&job.id))
                .await
                .expect("cancel")
                .status,
            DownloadJobStatus::Cancelled
        );
        assert_eq!(
            std::fs::read(save).expect("save preserved"),
            b"existing world"
        );
        assert_eq!(
            load_instance(&dir)
                .expect("instance preserved")
                .installation
                .expect("installation")
                .state,
            InstancePreparationState::Ready
        );
        assert!(!fixture.part().exists());
        drop(tail);
    }
}

#[cfg(windows)]
#[tokio::test]
async fn failed_instance_removal_can_retry_after_restart_without_the_profile() {
    use std::os::windows::fs::OpenOptionsExt;

    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Locked cancellation");
    let dir = fixture.game.instance_dir(&entry.folder).expect("folder");
    let locked = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .share_mode(0)
        .open(dir.join("locked.bin"))
        .expect("locked instance file");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    assert!(
        finish(manager.cancel(&job.id)).await.is_err(),
        "locked file must prevent removal"
    );
    assert!(
        list_download_jobs(&fixture.settings, &fixture.game)
            .expect("unfinished jobs")
            .iter()
            .any(|unfinished| unfinished.id == job.id),
        "failed cleanup must remain available for retry"
    );
    drop(locked);
    if dir.join("instance.json").exists() {
        std::fs::remove_file(dir.join("instance.json")).expect("partially removed profile");
    }
    drop(manager);
    crate::recover_download_jobs(&fixture.settings, &fixture.game).expect("restart recovery");
    let restarted = fixture.manager();
    let outcome = finish(restarted.cancel(&job.id))
        .await
        .expect("retry cleanup");
    assert_eq!(outcome.status, DownloadJobStatus::Cancelled);
    assert!(!dir.exists());
    assert!(
        server.requests.is_empty(),
        "retry must not reinstall the instance"
    );
    drop(tail);
}

#[tokio::test]
async fn pending_cancellation_recovers_after_cleanup_failure_with_missing_instance_files() {
    for remove_directory in [false, true] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url);
        let entry = fixture.instance("Pending cancellation");
        let dir = fixture.game.instance_dir(&entry.folder).expect("folder");
        let manager = fixture.manager();
        let job = manager
            .start(&entry.folder, DownloadJobOperation::Install)
            .await
            .expect("start");
        let tail = server.request().await.hold();
        let slot = manager.active_job(&job.id).await.expect("worker");
        let destination = fixture.game.root().join("blocked.bin");
        let obstruction = fixture.game.root().join("blocked.bin.part");
        super::super::jobs::register_artifacts(
            &fixture.settings,
            &fixture.game,
            &job.id,
            &[destination],
        )
        .expect("artifact");
        // A directory at the partial-file path makes cleanup fail on both supported OSes.
        std::fs::create_dir_all(&obstruction).expect("cleanup obstruction");
        assert!(manager.cancel(&job.id).await.is_err());
        assert!(
            finish(DownloadManager::wait_slot(&slot)).await.is_err(),
            "installation waiters must receive the cleanup failure"
        );
        let pending =
            get_download_job(&fixture.settings, &fixture.game, &job.id).expect("pending cleanup");
        assert_eq!(pending.status, DownloadJobStatus::Cancelling);
        assert!(pending.error.is_some());
        assert_eq!(
            list_download_jobs(&fixture.settings, &fixture.game).expect("visible job"),
            vec![pending.clone()]
        );
        assert!(
            manager.resume(&job.id).await.is_err(),
            "cleanup must not restart installation"
        );
        assert!(manager.ensure_launchable(&entry.folder).is_err());
        assert!(
            dir.join("instance.json").exists(),
            "artifact cleanup must finish before directory removal"
        );
        if remove_directory {
            std::fs::remove_dir_all(&dir).expect("directory already removed");
        } else {
            std::fs::remove_file(dir.join("instance.json")).expect("partially removed profile");
        }
        drop(manager);
        assert_eq!(
            crate::recover_download_jobs(&fixture.settings, &fixture.game).expect("recovery"),
            0
        );
        assert_eq!(
            get_download_job(&fixture.settings, &fixture.game, &job.id).expect("retained intent"),
            pending
        );
        let restarted = fixture.manager();
        assert!(
            restarted.cancel(&job.id).await.is_err(),
            "a failed retry must remain pending"
        );
        assert_eq!(
            get_download_job(&fixture.settings, &fixture.game, &job.id)
                .expect("still pending")
                .status,
            DownloadJobStatus::Cancelling
        );
        std::fs::remove_dir(&obstruction).expect("remove obstruction");
        assert_eq!(
            restarted.cancel(&job.id).await.expect("retry").status,
            DownloadJobStatus::Cancelled
        );
        assert!(!dir.exists());
        assert!(
            list_download_jobs(&fixture.settings, &fixture.game)
                .expect("finished cleanup")
                .is_empty()
        );
        assert!(server.requests.is_empty());
        drop(tail);
    }
}

#[tokio::test]
async fn cancelling_a_recovered_new_installation_removes_it_without_starting_network_requests() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Interrupted new instance");
    let job = create_pending_download_job(
        &fixture.settings,
        &fixture.game,
        &entry.folder,
        None,
        DownloadJobOperation::Install,
    )
    .expect("job");
    crate::recover_download_jobs(&fixture.settings, &fixture.game).expect("recovery");
    let manager = fixture.manager();
    assert_eq!(
        manager.cancel(&job.id).await.expect("cancel").status,
        DownloadJobStatus::Cancelled
    );
    assert!(
        !fixture
            .game
            .instance_dir(&entry.folder)
            .expect("folder")
            .exists()
    );
    assert!(server.requests.is_empty());
}

#[tokio::test]
async fn shutdown_preserves_pending_removal_when_the_cancel_caller_does_not_finish() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Pending shutdown");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    let slot = manager.active_job(&job.id).await.expect("worker");
    {
        let mut snapshot = slot.snapshot.lock().await;
        *snapshot = manager
            .prepare_cancellation(snapshot.clone())
            .expect("removal intent");
        slot.control.cancel();
    }
    // Model a cancel caller disappearing after the worker stops but before directory removal.
    assert_eq!(
        manager
            .cancel_job(&job.id)
            .await
            .expect("worker stopped")
            .status,
        DownloadJobStatus::Cancelling
    );
    finish(manager.suspend_all())
        .await
        .expect("shutdown must not wait for the missing caller");
    assert!(manager.active.lock().await.is_empty());
    assert_eq!(
        get_download_job(&fixture.settings, &fixture.game, &job.id)
            .expect("preserved intent")
            .status,
        DownloadJobStatus::Cancelling
    );
    assert!(DownloadManager::wait_slot(&slot).await.is_err());
    let restarted = fixture.manager();
    assert_eq!(
        restarted
            .cancel(&job.id)
            .await
            .expect("retry after restart")
            .status,
        DownloadJobStatus::Cancelled
    );
    assert!(
        !fixture
            .game
            .instance_dir(&entry.folder)
            .expect("folder")
            .exists()
    );
    drop(tail);
}

#[tokio::test]
async fn cancelling_an_existing_legacy_instance_update_keeps_its_files() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let root = fixture.game.instances_dir().expect("instances");
    let entry = crate::create_instance(
        &root,
        "Legacy",
        "1.20.1",
        &global_launch(&load_settings(&fixture.settings).expect("settings")),
    )
    .expect("legacy instance");
    let dir = fixture.game.instance_dir(&entry.folder).expect("folder");
    std::fs::write(dir.join("options.txt"), b"existing settings").expect("settings");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    finish(manager.cancel(&job.id)).await.expect("cancel");
    assert_eq!(
        std::fs::read(dir.join("options.txt")).expect("settings preserved"),
        b"existing settings"
    );
    assert!(
        load_instance(&dir)
            .expect("legacy profile")
            .installation
            .is_none()
    );
    drop(tail);
}

#[tokio::test]
async fn explicit_instance_deletion_cancels_downloads_without_deleting_the_folder_twice() {
    for pending_removal in [false, true] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url);
        let entry = fixture.instance("Delete");
        let manager = fixture.manager();
        let job = manager
            .start(&entry.folder, DownloadJobOperation::Install)
            .await
            .expect("start");
        let tail = server.request().await.hold();
        let slot = manager.active_job(&job.id).await.expect("worker");
        if pending_removal {
            let mut snapshot = slot.snapshot.lock().await;
            *snapshot = manager
                .prepare_cancellation(snapshot.clone())
                .expect("pending removal");
            slot.control.cancel();
        }
        finish(manager.cancel_instance(&entry.folder))
            .await
            .expect("stop downloads");
        assert_eq!(
            DownloadManager::wait_slot(&slot)
                .await
                .expect("cancellation outcome")
                .status,
            DownloadJobStatus::Cancelled
        );
        crate::delete_instance(&fixture.game.instances_dir().expect("root"), &entry.folder)
            .expect("delete once");
        assert!(
            !fixture
                .game
                .instance_dir(&entry.folder)
                .expect("folder")
                .exists()
        );
        drop(tail);
    }
}

#[tokio::test]
async fn a_failed_download_can_retry_the_same_job_and_clear_its_error() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let entry = fixture.instance("Retry");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Repair)
        .await
        .expect("start");
    // All retry attempts receive corrupt content; the job must stay resumable.
    for _ in 0..3 {
        server.request().await.reply(200, "", b"wrong bytes", 11);
    }
    let failed = finish(manager.wait(&job.id))
        .await
        .expect("failed snapshot");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert!(failed.error.is_some());
    let retry = manager.resume(&job.id).await.expect("retry");
    assert_eq!(retry.id, failed.id);
    assert!(retry.error.is_none());
    server.request().await.reply(200, "", BODY, 11);
    let complete = finish(manager.wait(&job.id)).await.expect("complete");
    assert_eq!(complete.status, DownloadJobStatus::Completed);
    assert!(complete.error.is_none());
}

#[tokio::test]
async fn standalone_java_has_no_fake_instance_and_cancellation_returns_no_path() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let manager = fixture.manager();
    let worker = Arc::clone(&manager);
    let install = tokio::spawn(async move { worker.install_java(21).await });
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    let job = list_download_jobs(&fixture.settings, &fixture.game)
        .expect("list")
        .pop()
        .expect("Java job");
    assert_eq!(job.instance_folder, None);
    assert_eq!(job.java_major, Some(21));
    finish(manager.cancel(&job.id)).await.expect("cancel");
    assert_eq!(
        finish(async { install.await.expect("join") })
            .await
            .expect("cancel result"),
        None
    );
    assert!(!fixture.game.instances_dir().expect("instances").exists());
    drop(tail);

    let worker = Arc::clone(&manager);
    let install = tokio::spawn(async move { worker.install_java(21).await });
    server.request().await.reply(200, "", BODY, 11);
    let path = finish(async { install.await.expect("join") })
        .await
        .expect("Java installation")
        .expect("path");
    assert!(Path::new(&path).is_file());
    assert_eq!(
        load_settings(&fixture.settings).expect("settings").java21,
        Some(path)
    );
}

#[tokio::test]
async fn recovered_cancellation_keeps_partials_referenced_by_another_unfinished_job() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused");
    let first = fixture.instance("First");
    let second = fixture.instance("Second");
    let first = create_pending_download_job(
        &fixture.settings,
        &fixture.game,
        &first.folder,
        Some("1.21.1"),
        DownloadJobOperation::Install,
    )
    .expect("first");
    let second = create_pending_download_job(
        &fixture.settings,
        &fixture.game,
        &second.folder,
        Some("1.21.1"),
        DownloadJobOperation::Install,
    )
    .expect("second");
    let destination = fixture.game.root().join("payload.bin");
    for job in [&first, &second] {
        super::super::jobs::register_artifacts(
            &fixture.settings,
            &fixture.game,
            &job.id,
            std::slice::from_ref(&destination),
        )
        .expect("artifact association");
    }
    std::fs::create_dir_all(fixture.game.root()).expect("data");
    std::fs::write(fixture.part(), b"hello").expect("partial");
    let manager = fixture.manager();
    manager.cancel(&first.id).await.expect("cancel first");
    assert!(fixture.part().exists());
    manager.cancel(&second.id).await.expect("cancel second");
    assert!(!fixture.part().exists());
}

#[tokio::test]
async fn cancelling_an_active_job_preserves_shared_cache_bytes_until_the_last_job_finishes() {
    for cancel_remaining in [false, true] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url);
        let first_instance = fixture.instance("First shared library consumer");
        let second_instance = fixture.instance("Second shared library consumer");
        let manager = fixture.manager();
        let first = manager
            .start(&first_instance.folder, DownloadJobOperation::Install)
            .await
            .expect("first install");
        let tail = server.request().await.hold();
        fixture.partial(5).await;
        let second = manager
            .start(&second_instance.folder, DownloadJobOperation::Install)
            .await
            .expect("second install");
        wait_until(|| {
            let connection = crate::open_db(&fixture.settings).expect("database");
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM download_job_artifacts WHERE job_id = ?1)",
                    [&second.id],
                    |row| row.get::<_, bool>(0),
                )
                .expect("second job artifact registered")
        })
        .await;
        manager
            .pause(&second.id)
            .await
            .expect("pause the waiting install");
        assert!(
            server.requests.is_empty(),
            "the second install must wait for the shared destination lock"
        );
        assert_eq!(
            finish(manager.cancel(&first.id))
                .await
                .expect("cancel first install")
                .status,
            DownloadJobStatus::Cancelled
        );
        assert_eq!(
            std::fs::read(fixture.part()).expect("shared cache partial must survive"),
            b"hello"
        );
        assert!(
            !fixture
                .game
                .instance_dir(&first_instance.folder)
                .expect("first folder")
                .exists()
        );
        assert!(
            fixture
                .game
                .instance_dir(&second_instance.folder)
                .expect("second folder")
                .exists()
        );
        if cancel_remaining {
            assert_eq!(
                finish(manager.cancel(&second.id))
                    .await
                    .expect("cancel last consumer")
                    .status,
                DownloadJobStatus::Cancelled
            );
            assert!(
                !fixture.part().exists(),
                "an unreferenced partial must be removed"
            );
            assert!(!fixture.game.root().join("payload.bin").exists());
        } else {
            manager
                .resume(&second.id)
                .await
                .expect("resume second install");
            let request = server.request().await;
            assert!(
                request
                    .headers
                    .to_ascii_lowercase()
                    .contains("range: bytes=5-"),
                "the surviving install must request only the remaining bytes"
            );
            request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
            assert_eq!(
                finish(manager.wait(&second.id))
                    .await
                    .expect("complete second install")
                    .status,
                DownloadJobStatus::Completed
            );
            assert_eq!(
                std::fs::read(fixture.game.root().join("payload.bin"))
                    .expect("verified shared file"),
                BODY
            );
            assert!(!fixture.part().exists());
        }
        assert!(server.requests.is_empty());
        drop(tail);
    }
}

#[tokio::test]
async fn accepted_cancellation_wins_even_if_an_executor_finishes_successfully() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url);
    let connection = crate::open_db(&fixture.settings).expect("database");
    crate::set_meta(&connection, "test_finish_after_cancel", "true").expect("completion gate");
    drop(connection);
    let entry = fixture.instance("Completion race");
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    server.request().await.reply(200, "", BODY, 11);
    wait_until(|| {
        load_instance(&fixture.game.instance_dir(&entry.folder).expect("folder")).is_ok_and(
            |profile| {
                profile.installation.is_some_and(|installation| {
                    installation.state == InstancePreparationState::Ready
                })
            },
        )
    })
    .await;
    let outcome = finish(manager.cancel(&job.id))
        .await
        .expect("cancel accepted before worker completion");
    assert_eq!(outcome.status, DownloadJobStatus::Cancelled);
    assert_eq!(
        get_download_job(&fixture.settings, &fixture.game, &job.id)
            .expect("persisted outcome")
            .status,
        DownloadJobStatus::Cancelled
    );
}

async fn finish<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .expect("operation timeout")
}

async fn wait_until(mut predicate: impl FnMut() -> bool) {
    finish(async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
}

struct Fixture {
    root: PathBuf,
    settings: SharedCache,
    game: SharedCache,
}
static FIXTURE_NUMBER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Fixture {
    fn new(url: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = FIXTURE_NUMBER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "licht-functional-download-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        let settings = SharedCache::at(root.join("settings"));
        let game = SharedCache::at(root.join("game").join("data"));
        let connection = crate::open_db(&settings).expect("database");
        crate::set_meta(&connection, "test_download_url", url).expect("fixture URL");
        Self {
            root,
            settings,
            game,
        }
    }

    fn instance(&self, name: &str) -> InstanceEntry {
        create_pending_instance(
            &self.game.instances_dir().expect("instances"),
            name,
            InstanceSource::Vanilla {
                game_version: "1.21.1".into(),
            },
            &global_launch(&load_settings(&self.settings).expect("settings")),
        )
        .expect("instance before network activity")
    }

    fn manager(&self) -> Arc<DownloadManager> {
        self.manager_with(fixture_executor, reqwest::Client::new())
    }

    fn manager_with(&self, executor: Executor, client: reqwest::Client) -> Arc<DownloadManager> {
        let mut manager = DownloadManager::new(
            self.settings.clone(),
            self.game.clone(),
            LaunchEnvironment {
                os: OsName::Linux,
                arch: Arch::X86_64,
                os_version: String::new(),
                features: BTreeMap::new(),
            },
            client,
        );
        manager.executor = executor;
        Arc::new(manager)
    }

    fn part(&self) -> PathBuf {
        self.game.root().join("payload.bin.part")
    }

    async fn partial(&self, bytes: u64) {
        wait_until(|| std::fs::metadata(self.part()).is_ok_and(|file| file.len() == bytes)).await;
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Do not mask an assertion failure with a secondary cleanup error.
        if let Err(error) = std::fs::remove_dir_all(&self.root)
            && !std::thread::panicking()
        {
            panic!("fixture cleanup failed: {error}");
        }
    }
}

struct Request {
    headers: String,
    response: oneshot::Sender<Reply>,
}
#[derive(Debug)]
struct Reply {
    bytes: Vec<u8>,
    tail: Option<oneshot::Receiver<Vec<u8>>>,
}

impl Request {
    fn reply(self, status: u16, headers: &str, body: &[u8], length: usize) {
        self.response
            .send(Reply::new(status, headers, body, length))
            .expect("response receiver");
    }

    fn hold(self) -> oneshot::Sender<Vec<u8>> {
        let (tail, receiver) = oneshot::channel();
        let mut reply = Reply::new(200, "", b"hello", 11);
        reply.tail = Some(receiver);
        self.response.send(reply).expect("response receiver");
        tail
    }
}

impl Reply {
    fn new(status: u16, headers: &str, body: &[u8], length: usize) -> Self {
        let mut bytes = format!("HTTP/1.1 {status} Response\r\nContent-Length: {length}\r\n{headers}Connection: close\r\n\r\n").into_bytes();
        bytes.extend_from_slice(body);
        Self { bytes, tail: None }
    }
}

struct Server {
    url: String,
    requests: mpsc::UnboundedReceiver<Request>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("http://{}/file", listener.local_addr().expect("address"));
        let (sender, requests) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let sender = sender.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = socket.read(&mut buffer).await.expect("headers");
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let (response, reply) = oneshot::channel();
                    sender
                        .send(Request {
                            headers: String::from_utf8(bytes).expect("headers"),
                            response,
                        })
                        .expect("request receiver");
                    let reply = reply.await.expect("reply");
                    if socket.write_all(&reply.bytes).await.is_err() {
                        return;
                    }
                    if let Some(tail) = reply.tail
                        && let Ok(bytes) = tail.await
                    {
                        // Pausing deliberately disconnects this socket.
                        let _disconnected_client = socket.write_all(&bytes).await;
                    }
                });
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }

    async fn request(&mut self) -> Request {
        finish(self.requests.recv()).await.expect("request")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
