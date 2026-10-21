use super::*;

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
