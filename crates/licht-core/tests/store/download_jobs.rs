use sqlx::Connection;

use licht_core::{
    Arch, CoreError, DownloadComponent, DownloadJobOperation, DownloadJobSnapshot,
    DownloadJobStatus, DownloadJobUpdate, DownloadManager, DownloadPhase, DownloadProgress,
    LaunchEnvironment, OsName, SharedCache, create_download_job, get_download_job,
    list_download_jobs, open_db, recover_download_jobs, update_download_job,
};

#[tokio::test]
async fn a_new_job_round_trips_as_a_queued_snapshot_with_a_string_id() {
    let fixture = Fixture::new("round-trip");
    let job = fixture.create("Survival").await;
    assert!(job.id.parse::<u64>().expect("numeric ID") > 0);
    assert_eq!(job.status, DownloadJobStatus::Queued);
    assert_eq!(job.operation, DownloadJobOperation::Install);
    assert_eq!(job.instance_folder.as_deref(), Some("Survival"));
    assert_eq!(job.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(job.phase, DownloadPhase::Preparing);
    assert_eq!(job.component, None);
    assert_eq!(job.progress, progress(0, 0, 0));
    assert!(job.created_at > 0);
    assert_eq!(job.updated_at, job.created_at);
    assert_eq!(fixture.get(&job.id).await, job);
    assert_eq!(fixture.list().await, vec![job.clone()]);

    let json = serde_json::to_value(job).expect("snapshot JSON");
    assert!(json["id"].is_string());
    assert_eq!(json["status"], "queued");
    assert_eq!(json["instanceFolder"], "Survival");
    assert!(json["createdAt"].is_number());
    assert!(json.get("dataDirectory").is_none());
}

#[tokio::test]
async fn phase_component_progress_and_lifecycle_survive_reopening_the_database() {
    let fixture = Fixture::new("lifecycle");
    let job = fixture.create("Fabric world").await;
    let running = fixture
        .update(&job.id, DownloadJobStatus::Running, progress(0, 5, 11))
        .await;
    assert_eq!(running.component, Some(DownloadComponent::Java));
    assert_eq!(running.phase, DownloadPhase::Downloading);
    let paused = fixture
        .update(&job.id, DownloadJobStatus::Paused, running.progress)
        .await;
    assert_eq!(fixture.get(&job.id).await, paused);
    assert!(paused.updated_at >= running.updated_at);

    fixture
        .update(&job.id, DownloadJobStatus::Running, paused.progress)
        .await;
    fixture
        .update(&job.id, DownloadJobStatus::Failed, progress(0, 5, 11))
        .await;
    fixture
        .update(&job.id, DownloadJobStatus::Queued, progress(0, 5, 11))
        .await;
    let extracting = update_download_job(
        &fixture.settings,
        &fixture.game,
        &job.id,
        DownloadJobUpdate {
            status: DownloadJobStatus::Running,
            component: Some(DownloadComponent::Fabric),
            phase: DownloadPhase::Extracting,
            progress: progress(1, 11, 11),
            error: None,
        },
    )
    .await
    .expect("next component");
    assert_eq!(fixture.get(&job.id).await, extracting);
    let completed = fixture
        .update(&job.id, DownloadJobStatus::Completed, extracting.progress)
        .await;
    assert_eq!(fixture.get(&job.id).await, completed);
    assert!(fixture.list().await.is_empty());
}

#[tokio::test]
async fn startup_recovery_interrupts_only_queued_and_running_jobs_in_the_selected_directory() {
    let fixture = Fixture::new("recovery");
    let queued = fixture.create("queued").await;
    let running = fixture.create("running").await;
    fixture
        .update(&running.id, DownloadJobStatus::Running, progress(0, 5, 11))
        .await;
    let paused = fixture.create("paused").await;
    fixture
        .update(&paused.id, DownloadJobStatus::Paused, progress(0, 5, 11))
        .await;
    let failed = fixture.create("failed").await;
    fixture
        .update(&failed.id, DownloadJobStatus::Failed, progress(0, 5, 11))
        .await;
    let complete = fixture.create("complete").await;
    fixture
        .update(
            &complete.id,
            DownloadJobStatus::Running,
            progress(1, 11, 11),
        )
        .await;
    fixture
        .update(
            &complete.id,
            DownloadJobStatus::Completed,
            progress(1, 11, 11),
        )
        .await;
    let cancelled = fixture.create("cancelled").await;
    fixture
        .update(
            &cancelled.id,
            DownloadJobStatus::Cancelled,
            progress(0, 0, 11),
        )
        .await;
    let other_game = SharedCache::at(fixture.settings.root().join("other-data"));
    let other = create_download_job(
        &fixture.settings,
        &other_game,
        "running",
        "1.21.1",
        DownloadJobOperation::Install,
    )
    .await
    .expect("other directory");
    let part = fixture.game.root().join("client.jar.part");
    std::fs::create_dir_all(fixture.game.root()).expect("data directory");
    std::fs::write(&part, b"hello").expect("partial");

    assert_eq!(
        recover_download_jobs(&fixture.settings, &fixture.game)
            .await
            .expect("recover"),
        2
    );
    assert_eq!(
        fixture.get(&queued.id).await.status,
        DownloadJobStatus::Interrupted
    );
    let recovered = fixture.get(&running.id).await;
    assert_eq!(recovered.status, DownloadJobStatus::Interrupted);
    assert_eq!(recovered.progress, progress(0, 5, 11));
    assert_eq!(recovered.component, Some(DownloadComponent::Java));
    assert_eq!(
        fixture.get(&paused.id).await.status,
        DownloadJobStatus::Paused
    );
    assert_eq!(
        fixture.get(&failed.id).await.status,
        DownloadJobStatus::Failed
    );
    assert_eq!(
        fixture.get(&complete.id).await.status,
        DownloadJobStatus::Completed
    );
    assert_eq!(
        fixture.get(&cancelled.id).await.status,
        DownloadJobStatus::Cancelled
    );
    assert_eq!(
        get_download_job(&fixture.settings, &other_game, &other.id)
            .await
            .expect("other")
            .status,
        DownloadJobStatus::Queued
    );
    assert_eq!(fixture.list().await.len(), 4);
    assert_eq!(
        recover_download_jobs(&fixture.settings, &fixture.game)
            .await
            .expect("repeat"),
        0
    );
    assert_eq!(std::fs::read(part).expect("preserved partial"), b"hello");
}

#[tokio::test]
async fn completed_and_cancelled_jobs_cannot_restart() {
    let fixture = Fixture::new("terminal");
    for status in [DownloadJobStatus::Completed, DownloadJobStatus::Cancelled] {
        let job = fixture.create(&format!("terminal-{status:?}")).await;
        fixture
            .update(&job.id, DownloadJobStatus::Running, progress(1, 11, 11))
            .await;
        let terminal = fixture.update(&job.id, status, progress(1, 11, 11)).await;
        let error = update_download_job(
            &fixture.settings,
            &fixture.game,
            &job.id,
            update(DownloadJobStatus::Running, progress(1, 11, 11)),
        )
        .await
        .expect_err("terminal state");
        assert!(matches!(error, CoreError::DownloadJobTransition));
        assert_eq!(fixture.get(&job.id).await, terminal);
    }
}

#[tokio::test]
async fn pending_cancellation_survives_recovery_and_cannot_restart_installation() {
    let fixture = Fixture::new("pending-cancellation");
    let job = fixture.create("Pending removal").await;
    let mut pending = update(DownloadJobStatus::Cancelling, progress(0, 0, 0));
    pending.error = Some("Instance file is locked".into());
    let stored = update_download_job(&fixture.settings, &fixture.game, &job.id, pending)
        .await
        .expect("cleanup pending");
    assert_eq!(
        recover_download_jobs(&fixture.settings, &fixture.game)
            .await
            .expect("recovery"),
        0
    );
    assert_eq!(fixture.list().await, vec![stored.clone()]);
    for next in [
        DownloadJobStatus::Queued,
        DownloadJobStatus::Running,
        DownloadJobStatus::Failed,
        DownloadJobStatus::Completed,
    ] {
        assert!(matches!(
            update_download_job(
                &fixture.settings,
                &fixture.game,
                &job.id,
                update(next, progress(0, 0, 0))
            )
            .await,
            Err(CoreError::DownloadJobTransition)
        ));
        assert_eq!(fixture.get(&job.id).await, stored);
    }
    let cancelled = fixture
        .update(&job.id, DownloadJobStatus::Cancelled, progress(0, 0, 0))
        .await;
    assert_eq!(cancelled.error, None);
    assert!(fixture.list().await.is_empty());
}

#[tokio::test]
async fn one_unfinished_job_per_instance_is_allowed_and_new_jobs_get_fresh_ids() {
    let fixture = Fixture::new("duplicate");
    let job = fixture.create("Survival").await;
    let error = create_download_job(
        &fixture.settings,
        &fixture.game,
        "Survival",
        "1.21.1",
        DownloadJobOperation::Repair,
    )
    .await
    .expect_err("duplicate job");
    assert!(matches!(error, CoreError::DownloadJobExists));
    fixture
        .update(&job.id, DownloadJobStatus::Cancelled, progress(0, 0, 0))
        .await;
    let repair = create_download_job(
        &fixture.settings,
        &fixture.game,
        "Survival",
        "1.21.1",
        DownloadJobOperation::Repair,
    )
    .await
    .expect("repair after cancellation");
    assert_ne!(job.id, repair.id);
    assert_eq!(repair.operation, DownloadJobOperation::Repair);
    assert_eq!(fixture.list().await, vec![repair]);
}

#[tokio::test]
async fn pending_instance_removal_rejects_repairs_and_standalone_java_tasks() {
    let fixture = Fixture::new("pending-removal-targets");
    let repair = create_download_job(
        &fixture.settings,
        &fixture.game,
        "Repair",
        "1.21.1",
        DownloadJobOperation::Repair,
    )
    .await
    .expect("repair");
    let java = fixture.create("Java target").await;
    let mut connection = open_db(&fixture.settings).await.expect("database");
    sqlx::query("UPDATE download_jobs SET instance_folder = NULL, java_major = 21 WHERE id = ?1")
        .bind(&java.id)
        .execute(&mut connection)
        .await
        .expect("standalone Java target");
    for id in [&repair.id, &java.id] {
        let before = fixture.get(id).await;
        assert!(matches!(
            update_download_job(
                &fixture.settings,
                &fixture.game,
                id,
                update(DownloadJobStatus::Cancelling, progress(0, 0, 0))
            )
            .await,
            Err(CoreError::DownloadJobInvalid { .. })
        ));
        assert_eq!(fixture.get(id).await, before);
    }
    sqlx::query("UPDATE download_jobs SET status = ?1 WHERE id = ?2")
        .bind("\"cancelling\"")
        .bind(repair.id)
        .execute(&mut connection)
        .await
        .expect("invalid persisted removal");
    assert!(matches!(
        list_download_jobs(&fixture.settings, &fixture.game).await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
    connection
        .close()
        .await
        .expect("close database before fixture cleanup");
}

#[tokio::test]
async fn concurrent_requests_cannot_create_duplicate_active_jobs() {
    let fixture = Fixture::new("concurrent");
    let barrier = tokio::sync::Barrier::new(4);
    let results = futures_util::future::join_all((0..4).map(|_| async {
        barrier.wait().await;
        create_download_job(
            &fixture.settings,
            &fixture.game,
            "Survival",
            "1.21.1",
            DownloadJobOperation::Install,
        )
        .await
    }))
    .await;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(CoreError::DownloadJobExists)))
            .count(),
        3
    );
    assert_eq!(fixture.list().await.len(), 1);
}

#[tokio::test]
async fn job_lookup_cannot_cross_data_directories_and_rejects_invalid_ids() {
    let fixture = Fixture::new("lookup");
    let job = fixture.create("Survival").await;
    let other_game = SharedCache::at(fixture.settings.root().join("other-data"));
    assert!(
        list_download_jobs(&fixture.settings, &other_game)
            .await
            .expect("other directory")
            .is_empty()
    );
    assert!(matches!(
        get_download_job(&fixture.settings, &other_game, &job.id).await,
        Err(CoreError::DownloadJobMissing)
    ));
    for id in ["", "0", "-1", "1 OR 1 = 1", "18446744073709551615"] {
        assert!(matches!(
            get_download_job(&fixture.settings, &fixture.game, id).await,
            Err(CoreError::DownloadJobInvalid { .. })
        ));
    }
    assert!(matches!(
        get_download_job(&fixture.settings, &fixture.game, "99999").await,
        Err(CoreError::DownloadJobMissing)
    ));
}

#[tokio::test]
async fn unsafe_contexts_and_invalid_progress_are_rejected_without_changing_the_job() {
    let fixture = Fixture::new("validation");
    for (folder, version) in [
        ("../escape", "1.21.1"),
        ("Survival", "../escape"),
        ("CON", "1.21.1"),
    ] {
        assert!(
            create_download_job(
                &fixture.settings,
                &fixture.game,
                folder,
                version,
                DownloadJobOperation::Install
            )
            .await
            .is_err()
        );
    }
    let relative = SharedCache::at("relative-data");
    assert!(matches!(
        create_download_job(
            &fixture.settings,
            &relative,
            "Survival",
            "1.21.1",
            DownloadJobOperation::Install
        )
        .await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
    let job = fixture.create("Survival").await;
    for progress in [
        progress(2, 0, 11),
        progress(0, 12, 11),
        DownloadProgress {
            finished: u32::MAX,
            failed: 1,
            ..progress(0, 0, 11)
        },
    ] {
        assert!(matches!(
            update_download_job(
                &fixture.settings,
                &fixture.game,
                &job.id,
                update(DownloadJobStatus::Running, progress)
            )
            .await,
            Err(CoreError::DownloadJobInvalid { .. })
        ));
        assert_eq!(fixture.get(&job.id).await, job);
    }
    let mut without_component = update(DownloadJobStatus::Running, progress(0, 0, 11));
    without_component.component = None;
    assert!(
        update_download_job(&fixture.settings, &fixture.game, &job.id, without_component)
            .await
            .is_err()
    );
    fixture
        .update(&job.id, DownloadJobStatus::Running, progress(0, 5, 11))
        .await;
    assert!(matches!(
        update_download_job(
            &fixture.settings,
            &fixture.game,
            &job.id,
            update(DownloadJobStatus::Completed, progress(0, 5, 11))
        )
        .await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
}

#[tokio::test]
async fn invalid_database_payloads_fail_explicitly_instead_of_disappearing_from_the_list() {
    let fixture = Fixture::new("corrupt-data");
    let job = fixture.create("Survival").await;
    let mut connection = open_db(&fixture.settings).await.expect("database");
    sqlx::query("UPDATE download_jobs SET progress = 'not JSON' WHERE id = ?1")
        .bind(&job.id)
        .execute(&mut connection)
        .await
        .expect("corrupt progress");
    assert!(matches!(
        list_download_jobs(&fixture.settings, &fixture.game).await,
        Err(CoreError::DownloadJobJson(_))
    ));
    sqlx::query("UPDATE download_jobs SET progress = ?1 WHERE id = ?2")
        .bind(serde_json::to_string(&progress(2, 0, 11)).expect("JSON"))
        .bind(job.id)
        .execute(&mut connection)
        .await
        .expect("invalid counts");
    assert!(matches!(
        list_download_jobs(&fixture.settings, &fixture.game).await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
    connection
        .close()
        .await
        .expect("close database before fixture cleanup");
}

#[tokio::test]
async fn indexed_cleanup_preserves_active_shared_partials_and_complete_files() {
    let fixture = Fixture::new("indexed-cleanup");
    let cancelled = create_download_job(
        &fixture.settings,
        &fixture.game,
        "Cancelled repair",
        "1.21.1",
        DownloadJobOperation::Repair,
    )
    .await
    .expect("repair job");
    let paused = fixture.create("Paused").await;
    fixture
        .update(&paused.id, DownloadJobStatus::Paused, progress(0, 5, 11))
        .await;
    let completed = fixture.create("Completed").await;
    fixture
        .update(
            &completed.id,
            DownloadJobStatus::Running,
            progress(1, 11, 11),
        )
        .await;
    fixture
        .update(
            &completed.id,
            DownloadJobStatus::Completed,
            progress(1, 11, 11),
        )
        .await;
    let mut connection = open_db(&fixture.settings).await.expect("database");
    for (id, name) in [
        (&cancelled.id, "exclusive.jar"),
        (&cancelled.id, "shared.jar"),
        (&cancelled.id, "completed.jar"),
        (&cancelled.id, "complete.jar"),
        (&paused.id, "shared.jar"),
        (&completed.id, "completed.jar"),
    ] {
        let path = fixture.game.root().join(name);
        sqlx::query("INSERT INTO download_job_artifacts (job_id, path) VALUES (?1, ?2)")
            .bind(id)
            .bind(path.to_str().expect("artifact path"))
            .execute(&mut connection)
            .await
            .expect("artifact association");
    }
    connection.close().await.expect("close fixture connection");
    std::fs::create_dir_all(fixture.game.root()).expect("game directory");
    for name in [
        "exclusive.jar.part",
        "shared.jar.part",
        "completed.jar.part",
    ] {
        std::fs::write(fixture.game.root().join(name), b"partial bytes").expect("partial");
    }
    let complete_file = fixture.game.root().join("complete.jar");
    std::fs::write(&complete_file, b"complete bytes").expect("complete file");
    let database = licht_core::DatabaseService::start(fixture.settings.clone())
        .await
        .expect("service");
    let manager = DownloadManager::new(
        fixture.settings.clone(),
        fixture.game.clone(),
        LaunchEnvironment {
            os: OsName::Linux,
            arch: Arch::X86_64,
            os_version: String::new(),
            features: Default::default(),
        },
        reqwest::Client::new(),
        database.client(),
    );
    let outcome = manager.cancel(&cancelled.id).await.expect("cancel repair");
    assert_eq!(outcome.status, DownloadJobStatus::Cancelled);
    assert!(!fixture.game.root().join("exclusive.jar.part").exists());
    assert!(!fixture.game.root().join("completed.jar.part").exists());
    assert_eq!(
        std::fs::read(fixture.game.root().join("shared.jar.part")).expect("active shared partial"),
        b"partial bytes"
    );
    assert_eq!(
        std::fs::read(complete_file).expect("complete file preserved"),
        b"complete bytes"
    );
    assert_eq!(
        fixture.get(&paused.id).await.status,
        DownloadJobStatus::Paused
    );
    assert_eq!(
        fixture.get(&completed.id).await.status,
        DownloadJobStatus::Completed
    );
    database.shutdown().await.expect("close service");
}

fn progress(finished: u32, bytes_done: u64, bytes_total: u64) -> DownloadProgress {
    DownloadProgress {
        finished,
        failed: 0,
        total: if bytes_total == 0 { 0 } else { 1 },
        bytes_done,
        bytes_total,
    }
}

fn update(status: DownloadJobStatus, progress: DownloadProgress) -> DownloadJobUpdate {
    DownloadJobUpdate {
        status,
        component: Some(DownloadComponent::Java),
        phase: DownloadPhase::Downloading,
        progress,
        error: None,
    }
}

struct Fixture {
    settings: SharedCache,
    game: SharedCache,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("licht-download-jobs-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture directory");
        Self {
            settings: SharedCache::at(root.join("settings")),
            game: SharedCache::at(root.join("data")),
        }
    }

    async fn create(&self, folder: &str) -> DownloadJobSnapshot {
        create_download_job(
            &self.settings,
            &self.game,
            folder,
            "1.21.1",
            DownloadJobOperation::Install,
        )
        .await
        .expect("create job")
    }

    async fn get(&self, id: &str) -> DownloadJobSnapshot {
        get_download_job(&self.settings, &self.game, id)
            .await
            .expect("load job")
    }

    async fn list(&self) -> Vec<DownloadJobSnapshot> {
        list_download_jobs(&self.settings, &self.game)
            .await
            .expect("list jobs")
    }

    async fn update(
        &self,
        id: &str,
        status: DownloadJobStatus,
        progress: DownloadProgress,
    ) -> DownloadJobSnapshot {
        update_download_job(&self.settings, &self.game, id, update(status, progress))
            .await
            .expect("update job")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.settings.root().parent().expect("fixture root"))
            .expect("cleanup");
    }
}
