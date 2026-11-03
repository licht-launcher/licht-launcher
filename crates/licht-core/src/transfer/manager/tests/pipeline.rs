use super::*;
use sha2::Digest;

const VERSION: &str = "1.21.11";

fn prepare_cache(fixture: &Fixture, server: &Server) -> Arc<DownloadManager> {
    fixture.game.create().expect("game cache");
    let index = r#"{"objects":{}}"#;
    let mut version: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/version-1.21.11.json"
    )))
    .expect("Mojang fixture");
    version["libraries"] = serde_json::json!([]);
    version["downloads"]["client"] =
        serde_json::json!({ "url": server.url, "sha1": SHA1, "size": BODY.len() });
    version["assetIndex"] = serde_json::json!({ "id": "pipeline-fixture", "sha1": crate::hashes::sha1_hex(index.as_bytes()), "size": index.len(), "totalSize": 0, "url": server.url });
    let version_path = fixture.game.version_json(VERSION).expect("version path");
    std::fs::create_dir_all(version_path.parent().expect("version directory")).expect("directory");
    std::fs::write(
        version_path,
        serde_json::to_vec(&version).expect("version JSON"),
    )
    .expect("cached version");
    std::fs::write(
        fixture
            .game
            .asset_index("pipeline-fixture")
            .expect("asset index"),
        index,
    )
    .expect("cached index");
    std::fs::write(crate::version_manifest_cache_path(&fixture.game), serde_json::to_vec(&serde_json::json!({ "latest": { "release": VERSION, "snapshot": VERSION }, "versions": [] })).expect("manifest JSON")).expect("fresh cached manifest");
    let bin = fixture
        .game
        .root()
        .join("runtime")
        .join("temurin21-jre21.0.4-linux_x64")
        .join("bin");
    std::fs::create_dir_all(&bin).expect("runtime directory");
    std::fs::write(bin.join("java"), b"fixture Java executable").expect("cached runtime");
    // Route even unexpected external URLs through the fixture server.
    let client = reqwest::Client::builder()
        .no_proxy()
        .proxy(reqwest::Proxy::all(server.url.trim_end_matches("/file")).expect("local proxy"))
        .build()
        .expect("fixture client");
    fixture.manager_with(crate::transfer::pipeline::execute, client)
}

async fn pending(fixture: &Fixture) -> InstanceEntry {
    create_pending_instance(
        &fixture.database(),
        &fixture.game.instances_dir().expect("instances"),
        "Real pipeline",
        InstanceSource::Vanilla {
            game_version: VERSION.into(),
        },
        &global_launch(&load_settings(&fixture.settings).expect("settings")),
    )
    .await
    .expect("pending instance")
}

async fn check_modpack_verification(
    state: InstancePreparationState,
    operation: DownloadJobOperation,
    expected: DownloadJobStatus,
) {
    for source in [
        crate::ModpackSource::Modrinth,
        crate::ModpackSource::Curseforge,
    ] {
        let server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let manager = prepare_cache(&fixture, &server);
        std::fs::write(
            fixture.game.version_jar(VERSION).expect("client path"),
            BODY,
        )
        .expect("cached client");
        let instance = pending(&fixture).await;
        let root = fixture.game.instances_dir().expect("instances");
        let database = fixture.database();
        if state != InstancePreparationState::Preparing {
            crate::configure_instance_version(&database, &root, &instance.folder, VERSION)
                .await
                .expect("configured version");
        }
        if state == InstancePreparationState::Ready {
            crate::complete_instance_preparation(&database, &root, &instance.folder)
                .await
                .expect("ready instance");
        }
        let directory = root.join(&instance.folder);
        let blocked = directory.join("mods/blocked.jar");
        // A directory cannot be hashed as a mod file, so verification fails before any download.
        std::fs::create_dir_all(&blocked).expect("unreadable mod fixture");
        std::fs::write(blocked.join("user.txt"), BODY).expect("preserved user file");
        match source {
            crate::ModpackSource::Modrinth => {
                let index = serde_json::json!({
                    "dependencies": {"minecraft": VERSION},
                    "files": [{
                        "path": "mods/blocked.jar", "fileSize": BODY.len(),
                        "hashes": {"sha1": SHA1, "sha512": "b".repeat(128)},
                        "downloads": ["https://cdn.modrinth.com/data/fixture/blocked.jar"]
                    }]
                });
                let stored =
                    crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory)
                        .expect("Modrinth index");
                database
                    .replace_modpack(&root, &instance.folder, stored)
                    .await
                    .expect("stored index");
            }
            crate::ModpackSource::Curseforge => {
                let index = serde_json::json!({
                    "source": "curseforge",
                    "files": [{
                        "path": "mods/blocked.jar", "fileSize": BODY.len(),
                        "hashes": {"sha1": SHA1},
                        "downloads": ["https://mediafilez.forgecdn.net/files/1/2/blocked.jar"],
                        "projectId": 1, "fileId": 2, "fileName": "blocked.jar",
                        "name": "Blocked fixture", "required": true
                    }]
                });
                std::fs::write(
                    directory.join(crate::CURSEFORGE_INDEX_FILE),
                    serde_json::to_vec(&index).expect("index JSON"),
                )
                .expect("CurseForge index");
            }
        }
        let outcome = finish(manager.prepare(&instance.folder, operation))
            .await
            .expect("pipeline outcome");
        assert_eq!(outcome.status, expected, "{source:?}: {:?}", outcome.error);
        if expected == DownloadJobStatus::Failed {
            assert_eq!(outcome.component, Some(crate::DownloadComponent::Modpack));
            assert_eq!(outcome.phase, DownloadPhase::Verifying);
            assert!(outcome.error.is_some());
        }
        assert_eq!(
            std::fs::read(blocked.join("user.txt")).expect("preserved file"),
            BODY
        );
        assert!(server.requests.is_empty());
        fixture.close().await;
    }
}

#[tokio::test]
async fn ready_instance_preparation_skips_modpack_verification() {
    check_modpack_verification(
        InstancePreparationState::Ready,
        DownloadJobOperation::Install,
        DownloadJobStatus::Completed,
    )
    .await;
}

#[tokio::test]
async fn first_installation_still_verifies_modpack_files() {
    check_modpack_verification(
        InstancePreparationState::Preparing,
        DownloadJobOperation::Install,
        DownloadJobStatus::Failed,
    )
    .await;
}

#[tokio::test]
async fn incomplete_installation_still_verifies_modpack_files() {
    check_modpack_verification(
        InstancePreparationState::Installing,
        DownloadJobOperation::Install,
        DownloadJobStatus::Failed,
    )
    .await;
}

#[tokio::test]
async fn manual_repair_still_verifies_ready_modpack_files() {
    check_modpack_verification(
        InstancePreparationState::Ready,
        DownloadJobOperation::Repair,
        DownloadJobStatus::Failed,
    )
    .await;
}

#[tokio::test]
async fn resuming_after_a_modpack_failure_reuses_completed_game_setup_after_reopening() {
    let server = Server::new().await;
    let mut fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("jar"), BODY).expect("cached client");
    let instance = pending(&fixture).await;
    let root = fixture.game.instances_dir().expect("instances");
    let directory = root.join(&instance.folder);
    let blocked = directory.join("mods/blocked.jar");
    std::fs::create_dir_all(&blocked).expect("unreadable mod");
    let index = serde_json::json!({"dependencies":{"minecraft":VERSION},"files":[{
        "path":"mods/blocked.jar", "fileSize":11,
        "hashes":{"sha1":SHA1,"sha512":crate::hashes::hex_encode(sha2::Sha512::digest(BODY).as_slice())},
        "downloads":["https://cdn.modrinth.com/data/fixture/blocked.jar"]
    }]});
    let stored =
        crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory).expect("index");
    fixture
        .database()
        .replace_modpack(&root, &instance.folder, stored)
        .await
        .expect("pack");
    let failed = finish(manager.prepare(&instance.folder, DownloadJobOperation::Install))
        .await
        .expect("job");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert_eq!(failed.component, Some(crate::DownloadComponent::Modpack));
    use crate::transfer::jobs::model::DownloadCheckpoint;
    assert!(
        fixture
            .database()
            .has_download_checkpoint(&fixture.game, &failed.id, VERSION, DownloadCheckpoint::Game)
            .await
            .expect("completed game")
    );
    assert!(
        !fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &failed.id,
                VERSION,
                DownloadCheckpoint::Modpack
            )
            .await
            .expect("unfinished pack")
    );
    drop(manager);
    fixture.reopen_database().await;
    // Completed game setup must not parse or hash these files again on this job's resume.
    std::fs::write(
        fixture.game.version_json(VERSION).expect("profile"),
        b"already completed game setup",
    )
    .expect("guard against repeated preparation");
    std::fs::remove_dir(&blocked).expect("unblock mod");
    std::fs::write(&blocked, BODY).expect("cached mod");
    let manager = fixture.manager_with(crate::transfer::pipeline::execute, reqwest::Client::new());
    let resumed = manager.resume(&failed.id).await.expect("resume");
    assert_eq!(resumed.id, failed.id);
    let completed = finish(manager.wait(&failed.id)).await.expect("completion");
    assert_eq!(
        completed.status,
        DownloadJobStatus::Completed,
        "{:?}",
        completed.error
    );
    assert!(
        fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &failed.id,
                VERSION,
                DownloadCheckpoint::Modpack
            )
            .await
            .expect("completed pack")
    );
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn a_new_repair_does_not_inherit_a_completed_jobs_checkpoints() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("jar"), BODY).expect("client");
    let instance = pending(&fixture).await;
    let installed = finish(manager.prepare(&instance.folder, DownloadJobOperation::Install))
        .await
        .expect("install");
    assert_eq!(installed.status, DownloadJobStatus::Completed);
    std::fs::write(
        fixture.game.version_json(VERSION).expect("profile"),
        b"invalid profile",
    )
    .expect("damaged profile");
    let repaired = finish(manager.prepare(&instance.folder, DownloadJobOperation::Repair))
        .await
        .expect("repair");
    assert_ne!(repaired.id, installed.id);
    assert_eq!(repaired.status, DownloadJobStatus::Failed);
    assert!(repaired.error.is_some());
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn repair_invalidates_native_cache_references_without_removing_existing_binaries() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("jar"), BODY).expect("client");
    let instance = pending(&fixture).await;
    assert_eq!(
        finish(manager.prepare(&instance.folder, DownloadJobOperation::Install))
            .await
            .expect("install")
            .status,
        DownloadJobStatus::Completed
    );
    let generation = "a".repeat(32);
    let fingerprint = "b".repeat(64);
    let directory = fixture
        .game
        .natives_dir(VERSION)
        .expect("natives")
        .join("linux-x86_64")
        .join(&generation);
    std::fs::create_dir_all(&directory).expect("generation");
    std::fs::write(directory.join("libnative.so"), b"in-use native").expect("existing binary");
    fixture
        .database()
        .store_native_generation(
            &fixture.game,
            VERSION,
            &fingerprint,
            crate::store::repositories::natives::NativeGeneration {
                generation,
                files: Vec::new(),
            },
        )
        .await
        .expect("native reference");
    let repaired = finish(manager.prepare(&instance.folder, DownloadJobOperation::Repair))
        .await
        .expect("repair");
    assert_eq!(
        repaired.status,
        DownloadJobStatus::Completed,
        "{:?}",
        repaired.error
    );
    assert!(
        fixture
            .database()
            .native_generation(&fixture.game, VERSION, &fingerprint)
            .await
            .expect("native cache")
            .is_none()
    );
    assert_eq!(
        std::fs::read(directory.join("libnative.so")).expect("old binary preserved"),
        b"in-use native"
    );
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn explicit_repair_rechecks_game_setup_after_a_pack_failure() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("client"), BODY)
        .expect("cached client");
    let instance = pending(&fixture).await;
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("directory");
    let index = directory.join(crate::CURSEFORGE_INDEX_FILE);
    std::fs::write(&index, b"invalid pack metadata").expect("pack failure");
    let failed = finish(manager.prepare(&instance.folder, DownloadJobOperation::Install))
        .await
        .expect("failed install");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert!(
        fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &failed.id,
                VERSION,
                crate::transfer::jobs::model::DownloadCheckpoint::Game
            )
            .await
            .expect("completed game")
    );
    std::fs::write(
        fixture.game.version_json(VERSION).expect("profile"),
        b"invalid game profile",
    )
    .expect("requires game verification");
    std::fs::write(&index, br#"{"source":"curseforge","files":[]}"#).expect("fixed pack");
    let repaired = finish(manager.prepare(&instance.folder, DownloadJobOperation::Repair))
        .await
        .expect("repair");
    assert_eq!(repaired.id, failed.id);
    assert_eq!(repaired.operation, DownloadJobOperation::Repair);
    assert_eq!(
        repaired.status,
        DownloadJobStatus::Failed,
        "Repair must re-read the game profile instead of reusing its checkpoint"
    );
    assert!(
        repaired
            .error
            .as_deref()
            .is_some_and(|error| error.contains("version JSON is invalid"))
    );
    assert!(
        !fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &failed.id,
                VERSION,
                crate::transfer::jobs::model::DownloadCheckpoint::Game
            )
            .await
            .expect("old checkpoint cleared")
    );
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn resuming_finalization_reuses_a_completed_modpack_stage() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("jar"), BODY).expect("client");
    let instance = pending(&fixture).await;
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("directory");
    let archive = directory.join(".install").join("pack.zip");
    std::fs::create_dir_all(&archive).expect("block final cleanup");
    let failed = finish(manager.prepare(&instance.folder, DownloadJobOperation::Install))
        .await
        .expect("installation");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    use crate::transfer::jobs::model::DownloadCheckpoint;
    assert!(
        fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &failed.id,
                VERSION,
                DownloadCheckpoint::Modpack
            )
            .await
            .expect("pack stage completed")
    );
    std::fs::remove_dir(archive).expect("unblock cleanup");
    // Reopening an already completed pack stage would try to parse this file.
    std::fs::write(
        directory.join(crate::CURSEFORGE_INDEX_FILE),
        b"completed pack stage",
    )
    .expect("guard against repeated verification");
    manager.resume(&failed.id).await.expect("resume");
    let completed = finish(manager.wait(&failed.id)).await.expect("completion");
    assert_eq!(
        completed.status,
        DownloadJobStatus::Completed,
        "{:?}",
        completed.error
    );
    assert!(!directory.join(".install").exists());
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn real_vanilla_pipeline_pauses_resumes_and_publishes_a_ready_instance() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    let instance = pending(&fixture).await;
    let worker = Arc::clone(&manager);
    let folder = instance.folder.clone();
    let preparation =
        tokio::spawn(async move { worker.prepare(&folder, DownloadJobOperation::Install).await });
    let tail = server.request().await.hold();
    let part = fixture
        .game
        .version_jar(VERSION)
        .expect("client path")
        .with_file_name(format!("{VERSION}.jar.part"));
    wait_until(|| std::fs::metadata(&part).is_ok_and(|file| file.len() == 5)).await;
    let job = list_download_jobs(&fixture.settings, &fixture.game)
        .await
        .expect("jobs")
        .pop()
        .expect("job");
    manager.pause(&job.id).await.expect("pause");
    assert!(manager.ensure_launchable(&instance.folder).await.is_err());
    assert_eq!(
        load_instance(
            &fixture.database(),
            &fixture.game.instance_dir(&instance.folder).expect("folder")
        )
        .await
        .expect("profile")
        .installation
        .expect("installation")
        .state,
        InstancePreparationState::Installing
    );
    manager.resume(&job.id).await.expect("resume");
    let request = server.request().await;
    assert!(
        request
            .headers
            .to_ascii_lowercase()
            .contains("range: bytes=5-")
    );
    request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
    assert_eq!(
        finish(async { preparation.await.expect("worker") })
            .await
            .expect("installation")
            .status,
        DownloadJobStatus::Completed
    );
    assert_eq!(
        std::fs::read(fixture.game.version_jar(VERSION).expect("client")).expect("verified client"),
        BODY
    );
    manager
        .ensure_launchable(&instance.folder)
        .await
        .expect("ready instance");
    assert!(!part.exists());
    assert!(server.requests.is_empty());
    drop(tail);
    fixture.close().await;
}

#[tokio::test]
async fn real_pipeline_checksum_failure_keeps_the_instance_unlaunchable() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    let instance = pending(&fixture).await;
    let worker = Arc::clone(&manager);
    let folder = instance.folder.clone();
    let preparation =
        tokio::spawn(async move { worker.prepare(&folder, DownloadJobOperation::Install).await });
    for _ in 0..3 {
        server.request().await.reply(200, "", b"wrong bytes", 11);
    }
    let outcome = finish(async { preparation.await.expect("worker") })
        .await
        .expect("failed snapshot");
    assert_eq!(outcome.status, DownloadJobStatus::Failed);
    assert!(outcome.error.is_some());
    assert!(
        !fixture
            .database()
            .has_download_checkpoint(
                &fixture.game,
                &outcome.id,
                VERSION,
                crate::transfer::jobs::model::DownloadCheckpoint::Game
            )
            .await
            .expect("failed game is not checkpointed")
    );
    assert!(!fixture.game.version_jar(VERSION).expect("client").exists());
    assert!(manager.ensure_launchable(&instance.folder).await.is_err());
    assert_eq!(
        get_download_job(&fixture.settings, &fixture.game, &outcome.id)
            .await
            .expect("persisted failure"),
        outcome
    );
    fixture.close().await;
}

#[tokio::test]
async fn real_pipeline_cancellation_stops_installation_before_removing_the_instance() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    let instance = pending(&fixture).await;
    let worker = Arc::clone(&manager);
    let folder = instance.folder.clone();
    let preparation =
        tokio::spawn(async move { worker.prepare(&folder, DownloadJobOperation::Install).await });
    let tail = server.request().await.hold();
    let job = list_download_jobs(&fixture.settings, &fixture.game)
        .await
        .expect("jobs")
        .pop()
        .expect("job");
    assert_eq!(
        finish(manager.cancel(&job.id))
            .await
            .expect("cancel")
            .status,
        DownloadJobStatus::Cancelled
    );
    assert_eq!(
        finish(async { preparation.await.expect("worker") })
            .await
            .expect("cancelled installation")
            .status,
        DownloadJobStatus::Cancelled
    );
    assert!(
        !fixture
            .game
            .instance_dir(&instance.folder)
            .expect("folder")
            .exists()
    );
    assert!(!fixture.game.version_jar(VERSION).expect("client").exists());
    assert!(
        list_download_jobs(&fixture.settings, &fixture.game)
            .await
            .expect("jobs")
            .is_empty()
    );
    drop(tail);
    fixture.close().await;
}

#[tokio::test]
async fn real_pipeline_resumes_a_persisted_loader_profile_without_reselecting_it() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    let loader = "fabric-loader-0.16.10-1.21.11";
    let source = InstanceSource::Loader {
        platform: crate::LoaderPlatform::Fabric,
        game_version: VERSION.into(),
        channel: crate::LoaderChannel::Manual,
        loader_version: Some("0.16.10".into()),
    };
    let root = fixture.game.instances_dir().expect("instances");
    let instance = create_pending_instance(
        &fixture.database(),
        &root,
        "Persisted loader",
        source.clone(),
        &global_launch(&load_settings(&fixture.settings).expect("settings")),
    )
    .await
    .expect("pending loader");
    let profile = fixture.game.version_json(loader).expect("loader path");
    std::fs::create_dir_all(profile.parent().expect("profile directory")).expect("directory");
    std::fs::write(profile, serde_json::to_vec(&serde_json::json!({ "id": loader, "inheritsFrom": VERSION, "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient", "libraries": [] })).expect("loader JSON")).expect("cached profile");
    crate::configure_instance_version(&fixture.database(), &root, &instance.folder, loader)
        .await
        .expect("resolved loader");
    let preparation = manager.prepare(&instance.folder, DownloadJobOperation::Install);
    tokio::pin!(preparation);
    let outcome = finish(async {
        loop {
            tokio::select! {
                result = &mut preparation => break result,
                request = server.requests.recv() => {
                    let request = request.expect("client request");
                    assert!(request.headers.starts_with("GET "), "unexpected metadata tunnel: {}", request.headers);
                    request.reply(200, "", BODY, BODY.len());
                }
            }
        }
    }).await.expect("loader installation");
    assert_eq!(outcome.status, DownloadJobStatus::Completed);
    assert_eq!(outcome.version_id.as_deref(), Some(loader));
    let installed = load_instance(
        &fixture.database(),
        &fixture.game.instance_dir(&instance.folder).expect("folder"),
    )
    .await
    .expect("profile");
    let installation = installed.installation.expect("installation");
    assert_eq!(installation.source, source);
    assert_eq!(installation.state, InstancePreparationState::Ready);
    manager
        .ensure_launchable(&instance.folder)
        .await
        .expect("ready loader");
    fixture.close().await;
}

fn local_mrpack() -> Vec<u8> {
    use std::io::Write;
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("modrinth.index.json", options)
            .expect("index entry");
        zip.write_all(
            serde_json::json!({"dependencies": {"minecraft": VERSION}, "files": []})
                .to_string()
                .as_bytes(),
        )
        .expect("index");
        zip.start_file("overrides/options.txt", options)
            .expect("override entry");
        zip.write_all(b"fov:90").expect("override");
        zip.finish().expect("archive");
    }
    buffer.into_inner()
}

#[tokio::test]
async fn creating_from_a_local_modpack_archive_installs_it_without_network_requests() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    std::fs::write(fixture.game.version_jar(VERSION).expect("jar"), BODY).expect("cached client");
    let archive = fixture.game.root().join("local.mrpack");
    std::fs::write(&archive, local_mrpack()).expect("local archive");
    let entry = manager
        .create_from_modpack_file("Local pack", &archive)
        .await
        .expect("instance from archive");
    let job = manager
        .stored_jobs()
        .await
        .expect("jobs")
        .into_iter()
        .find(|job| job.instance_folder.as_deref() == Some(entry.folder.as_str()))
        .expect("installation job");
    let done = finish(manager.wait(&job.id)).await.expect("completion");
    assert_eq!(
        done.status,
        DownloadJobStatus::Completed,
        "{:?}",
        done.error
    );
    let directory = fixture
        .game
        .instances_dir()
        .expect("instances")
        .join(&entry.folder);
    assert_eq!(
        std::fs::read_to_string(directory.join("options.txt")).expect("override"),
        "fov:90"
    );
    let profile = crate::load_instance(&fixture.database(), &directory)
        .await
        .expect("profile");
    assert_eq!(profile.version_id.as_deref(), Some(VERSION));
    assert_eq!(
        profile.installation.map(|installation| installation.source),
        Some(InstanceSource::ModpackFile {
            source: crate::ModpackSource::Modrinth
        })
    );
    assert!(archive.is_file(), "the original archive must be untouched");
    assert!(server.requests.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn creating_from_an_unrecognized_archive_leaves_no_instance_behind() {
    let server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let manager = prepare_cache(&fixture, &server);
    let archive = fixture.game.root().join("not-a-pack.zip");
    std::fs::write(&archive, b"definitely not a zip").expect("bad archive");
    let error = manager
        .create_from_modpack_file("Broken pack", &archive)
        .await
        .expect_err("invalid archive");
    assert!(matches!(error, CoreError::ModpackArchive), "{error:?}");
    let instances = fixture.game.instances_dir().expect("instances");
    let leftovers = std::fs::read_dir(&instances).map_or(0, Iterator::count);
    assert_eq!(leftovers, 0);
    fixture.close().await;
}
