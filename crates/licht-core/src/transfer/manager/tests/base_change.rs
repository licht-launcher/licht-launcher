use super::*;

/// A ready Vanilla 1.21.1 instance.
async fn ready_vanilla(fixture: &Fixture, name: &str) -> InstanceEntry {
    let entry = fixture.instance(name).await;
    let root = fixture.game.instances_dir().expect("instances");
    crate::configure_instance_version(&fixture.database(), &root, &entry.folder, "1.21.1")
        .await
        .expect("version");
    crate::complete_instance_preparation(&fixture.database(), &root, &entry.folder)
        .await
        .expect("ready")
}

fn fabric() -> InstanceSource {
    InstanceSource::Loader {
        platform: crate::LoaderPlatform::Fabric,
        game_version: "1.21.4".into(),
        channel: crate::LoaderChannel::Manual,
        loader_version: Some("0.16.10".into()),
    }
}

async fn profile(fixture: &Fixture, folder: &str) -> crate::Instance {
    crate::load_instance(
        &fixture.database(),
        &fixture.game.instance_dir(folder).expect("dir"),
    )
    .await
    .expect("profile")
}

#[tokio::test]
async fn cancelling_a_base_change_puts_the_previous_base_back() {
    // No server answers, so the change job fails and waits to be retried or cancelled.
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_vanilla(&fixture, "Survival").await;

    let failed = manager
        .change_base(&entry.folder, fabric())
        .await
        .expect("change job");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert_eq!(failed.operation, DownloadJobOperation::ChangeBase);
    let waiting = profile(&fixture, &entry.folder).await;
    let installation = waiting.installation.expect("installation");
    assert_eq!(installation.source, fabric());
    assert_eq!(installation.state, InstancePreparationState::Preparing);

    let cancelled = manager.cancel(&failed.id).await.expect("cancel");

    assert_eq!(cancelled.status, DownloadJobStatus::Cancelled);
    let restored = profile(&fixture, &entry.folder).await;
    assert_eq!(restored.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(
        restored.installation,
        Some(crate::InstanceInstallation {
            source: InstanceSource::Vanilla {
                game_version: "1.21.1".into(),
            },
            state: InstancePreparationState::Ready,
            modpack: None,
            previous_base: None,
        })
    );
    assert!(
        fixture
            .game
            .instance_dir(&entry.folder)
            .expect("dir")
            .is_dir(),
        "cancelling a change never removes the instance"
    );
    fixture.close().await;
}

#[tokio::test]
async fn a_base_change_waits_for_other_tasks_and_a_closed_game() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let busy = ready_vanilla(&fixture, "Busy").await;
    fixture
        .database()
        .create_pending_download_job(
            &fixture.game,
            &busy.folder,
            Some("1.21.1"),
            DownloadJobOperation::Repair,
        )
        .await
        .expect("unfinished repair");
    let running = ready_vanilla(&fixture, "Running").await;
    let dir = fixture.game.instance_dir(&running.folder).expect("dir");
    // This test process stands in for the running game.
    crate::write_launch_lock_for_pid(&dir, std::process::id()).expect("launch lock");

    assert!(matches!(
        manager.change_base(&busy.folder, fabric()).await,
        Err(CoreError::DownloadJobExists)
    ));
    assert!(matches!(
        manager.change_base(&running.folder, fabric()).await,
        Err(CoreError::InstanceRunning)
    ));
    for folder in [&busy.folder, &running.folder] {
        let unchanged = profile(&fixture, folder).await;
        assert_eq!(unchanged.version_id.as_deref(), Some("1.21.1"));
        assert_eq!(
            unchanged
                .installation
                .map(|installation| installation.state),
            Some(InstancePreparationState::Ready)
        );
    }
    assert_eq!(manager.stored_jobs().await.expect("jobs").len(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn a_repair_never_replaces_a_waiting_base_change() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_vanilla(&fixture, "Changing").await;
    let failed = manager
        .change_base(&entry.folder, fabric())
        .await
        .expect("change job");
    assert_eq!(failed.status, DownloadJobStatus::Failed);

    assert!(matches!(
        manager
            .prepare(&entry.folder, DownloadJobOperation::Repair)
            .await,
        Err(CoreError::DownloadJobExists)
    ));
    fixture.close().await;
}

/// A ready instance linked to version `v15` of a Modrinth pack.
async fn ready_store_pack(fixture: &Fixture, name: &str) -> InstanceEntry {
    let database = fixture.database();
    let root = fixture.game.instances_dir().expect("instances");
    let template = crate::global_launch(
        &crate::load_settings(&fixture.settings)
            .await
            .expect("settings"),
    );
    let entry =
        crate::create_pending_instance(&database, &root, name, store_pack("v15"), &template)
            .await
            .expect("instance");
    crate::instance::record_linked_modpack(&database, &root, &entry.folder, linked("15.0"))
        .await
        .expect("details");
    crate::configure_instance_version(
        &database,
        &root,
        &entry.folder,
        "fabric-loader-0.16.10-1.21.1",
    )
    .await
    .expect("version");
    crate::complete_instance_preparation(&database, &root, &entry.folder)
        .await
        .expect("ready")
}

fn store_pack(version_id: &str) -> InstanceSource {
    InstanceSource::Modpack {
        source: crate::ModpackSource::Modrinth,
        project_id: "fo".into(),
        version_id: version_id.into(),
        icon_url: None,
    }
}

fn linked(version_name: &str) -> crate::LinkedModpack {
    crate::LinkedModpack {
        name: "Fabulously Optimized".into(),
        author: Some("robotkoer".into()),
        version_name: version_name.into(),
        published_at: None,
    }
}

#[tokio::test]
async fn cancelling_a_modpack_version_change_puts_the_previous_version_back() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_store_pack(&fixture, "Pack").await;

    let failed = manager
        .change_modpack_version(&entry.folder, "v16")
        .await
        .expect("change job");
    assert_eq!(failed.operation, DownloadJobOperation::ChangeVersion);
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert!(matches!(
        crate::instance_version_for_launch(&profile(&fixture, &entry.folder).await),
        Err(CoreError::InstancePreparing)
    ));
    let waiting = profile(&fixture, &entry.folder).await;
    assert_eq!(
        waiting.installation.map(|installation| installation.source),
        Some(store_pack("v16"))
    );
    assert!(matches!(
        manager
            .prepare(&entry.folder, DownloadJobOperation::Repair)
            .await,
        Err(CoreError::DownloadJobExists)
    ));

    manager.cancel(&failed.id).await.expect("cancel");

    let restored = profile(&fixture, &entry.folder).await;
    assert_eq!(
        restored.version_id.as_deref(),
        Some("fabric-loader-0.16.10-1.21.1")
    );
    assert_eq!(
        restored.installation,
        Some(crate::InstanceInstallation {
            source: store_pack("v15"),
            state: InstancePreparationState::Ready,
            modpack: Some(linked("15.0")),
            previous_base: None,
        })
    );
    fixture.close().await;
}

#[tokio::test]
async fn only_a_store_pack_can_change_its_version() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let vanilla = ready_vanilla(&fixture, "Vanilla").await;
    let root = fixture.game.instances_dir().expect("instances");
    let template = crate::global_launch(
        &crate::load_settings(&fixture.settings)
            .await
            .expect("settings"),
    );
    let file = crate::create_pending_instance(
        &fixture.database(),
        &root,
        "File pack",
        InstanceSource::ModpackFile {
            source: crate::ModpackSource::Modrinth,
        },
        &template,
    )
    .await
    .expect("instance");
    crate::configure_instance_version(&fixture.database(), &root, &file.folder, "1.21.1")
        .await
        .expect("version");
    crate::complete_instance_preparation(&fixture.database(), &root, &file.folder)
        .await
        .expect("ready");
    let store = ready_store_pack(&fixture, "Store pack").await;

    assert!(matches!(
        manager.change_modpack_version(&vanilla.folder, "v16").await,
        Err(CoreError::InstanceNotLinked)
    ));
    assert!(matches!(
        manager.change_modpack_version(&file.folder, "v16").await,
        Err(CoreError::ModpackFileVersion)
    ));
    assert!(matches!(
        manager
            .change_modpack_version(&store.folder, "../v16")
            .await,
        Err(CoreError::ModpackMeta { .. })
    ));
    assert!(matches!(
        manager.change_base(&store.folder, fabric()).await,
        Err(CoreError::InstanceLinked)
    ));
    assert!(manager.stored_jobs().await.expect("jobs").is_empty());
    assert_eq!(
        profile(&fixture, &store.folder)
            .await
            .installation
            .map(|installation| installation.source),
        Some(store_pack("v15"))
    );
    fixture.close().await;
}
/// Leaves `folder` mid change with no job, as a crash between recording the change
/// and creating its job would.
async fn change_without_a_job(fixture: &Fixture, folder: &str) {
    crate::begin_instance_base_change(
        &fixture.database(),
        &fixture.game.instances_dir().expect("instances"),
        folder,
        fabric(),
    )
    .await
    .expect("begin");
}

#[tokio::test]
async fn preparing_an_instance_left_mid_change_resumes_the_change() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_vanilla(&fixture, "Interrupted").await;
    change_without_a_job(&fixture, &entry.folder).await;

    let resumed = manager
        .prepare(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("prepare");
    assert_eq!(resumed.operation, DownloadJobOperation::ChangeBase);

    manager.cancel(&resumed.id).await.expect("cancel");
    let restored = profile(&fixture, &entry.folder).await;
    assert_eq!(restored.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(
        restored.installation.map(|installation| installation.state),
        Some(InstancePreparationState::Ready)
    );
    fixture.close().await;
}

#[tokio::test]
async fn cancelling_an_install_never_removes_an_instance_mid_change() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_vanilla(&fixture, "Kept").await;
    change_without_a_job(&fixture, &entry.folder).await;
    let install = fixture
        .database()
        .create_pending_download_job(
            &fixture.game,
            &entry.folder,
            Some("1.21.1"),
            DownloadJobOperation::Install,
        )
        .await
        .expect("install job");

    let cancelled = manager.cancel(&install.id).await.expect("cancel");

    assert_eq!(cancelled.status, DownloadJobStatus::Cancelled);
    assert!(
        fixture
            .game
            .instance_dir(&entry.folder)
            .expect("dir")
            .is_dir(),
        "the existing instance stays"
    );
    let restored = profile(&fixture, &entry.folder).await;
    assert_eq!(restored.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(
        restored.installation.map(|installation| installation.state),
        Some(InstancePreparationState::Ready)
    );
    fixture.close().await;
}

#[tokio::test]
async fn a_change_or_reinstall_starts_only_from_its_own_action() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_vanilla(&fixture, "Guarded").await;

    for operation in [
        DownloadJobOperation::ChangeBase,
        DownloadJobOperation::ChangeVersion,
        DownloadJobOperation::Reinstall,
    ] {
        assert!(matches!(
            manager.prepare(&entry.folder, operation).await,
            Err(CoreError::DownloadOperationUnsupported)
        ));
    }
    assert!(manager.stored_jobs().await.expect("jobs").is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn a_version_change_past_its_content_waits_to_finish_instead_of_going_back() {
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let manager = fixture.manager();
    let entry = ready_store_pack(&fixture, "Committed").await;
    let root = fixture.game.instances_dir().expect("instances");
    crate::begin_modpack_version_change(&fixture.database(), &root, &entry.folder, "v16")
        .await
        .expect("begin");
    // The job started replacing the pack's content before it stopped.
    crate::instance::commit_modpack_content_change(
        &fixture.database(),
        &root,
        &entry.folder,
        vec!["mods/sodium.jar".into()],
    )
    .await
    .expect("commit");

    let resumed = manager
        .prepare(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("prepare");
    assert_eq!(resumed.operation, DownloadJobOperation::ChangeVersion);
    manager.cancel(&resumed.id).await.expect("cancel");

    let waiting = profile(&fixture, &entry.folder).await;
    let installation = waiting.installation.expect("installation");
    assert_eq!(installation.source, store_pack("v16"));
    assert_eq!(installation.state, InstancePreparationState::Installing);
    assert_eq!(
        installation
            .previous_base
            .and_then(|previous| previous.disabled_content),
        Some(vec!["mods/sodium.jar".to_string()])
    );
    fixture.close().await;
}
