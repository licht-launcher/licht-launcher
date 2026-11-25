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
