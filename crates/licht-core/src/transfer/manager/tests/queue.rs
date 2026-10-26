use super::*;

fn queue_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let folder = job
            .instance_folder
            .as_deref()
            .ok_or(CoreError::InstancePreparing)?;
        context.stage(DownloadPhase::Downloading).await?;
        std::fs::create_dir_all(runtime.game.root())?;
        std::fs::write(
            runtime.game.root().join(format!("{folder}.started")),
            b"started",
        )?;
        loop {
            context.checkpoint().await?;
            if runtime.game.root().join(format!("{folder}.fail")).exists() {
                return Err(CoreError::DownloadJobExecution {
                    detail: "fixture installation failed".into(),
                });
            }
            if runtime
                .game
                .root()
                .join(format!("{folder}.finish"))
                .exists()
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
}

fn started(fixture: &Fixture, entry: &InstanceEntry) -> bool {
    fixture
        .game
        .root()
        .join(format!("{}.started", entry.folder))
        .exists()
}

#[tokio::test]
async fn resuming_an_item_does_not_report_a_user_pause_while_reordering_is_blocked() {
    use sqlx::Connection;
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    manager.pause(&b.id).await.expect("pause waiting job");
    let mut database = crate::open_db(&fixture.settings).await.expect("database");
    let lock = database
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("hold writer");
    let owner = Arc::clone(&manager);
    let id = b.id.clone();
    let resuming = tokio::spawn(async move { owner.resume(&id).await });
    wait_until(|| manager.starts.try_lock().is_err()).await;
    assert!(!manager.telemetry().paused);
    lock.commit().await.expect("release writer");
    finish(resuming)
        .await
        .expect("resume worker")
        .expect("resume job");
    database.close().await.expect("close database");
    signal(&fixture, &first, "finish");
    finish(manager.wait(&a.id)).await.expect("first completed");
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second completed");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn pausing_the_last_running_job_keeps_it_ahead_of_older_paused_items() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    manager.pause(&a.id).await.expect("pause first");
    wait_until(|| started(&fixture, &second)).await;
    manager.pause(&b.id).await.expect("pause last running");
    let jobs = manager.stored_jobs().await.expect("paused queue");
    assert_eq!(jobs.first().expect("last active").id, b.id);
    assert!(
        jobs.iter()
            .all(|job| job.status == DownloadJobStatus::Paused)
    );
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn resuming_a_reordered_paused_job_preserves_its_position_among_queued_jobs() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    manager.pause(&c.id).await.expect("pause waiting third");
    manager
        .reorder(vec![c.id.clone(), b.id.clone()])
        .await
        .expect("move paused job first");
    assert!(!started(&fixture, &third));
    manager
        .resume(&c.id)
        .await
        .expect("resume third in its chosen position");
    signal(&fixture, &first, "finish");
    finish(manager.wait(&a.id)).await.expect("first completed");
    wait_until(|| started(&fixture, &third)).await;
    assert!(!started(&fixture, &second));
    signal(&fixture, &third, "finish");
    finish(manager.wait(&c.id)).await.expect("third completed");
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second completed");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn paused_queue_preserves_the_last_active_and_manual_order_after_restart() {
    let mut fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    manager.pause(&a.id).await.expect("pause first");
    wait_until(|| started(&fixture, &second)).await;
    manager.pause_all().await.expect("pause queue");
    let paused = manager.stored_jobs().await.expect("paused jobs");
    assert_eq!(paused.first().expect("last active").id, b.id);
    assert!(
        paused
            .iter()
            .all(|job| job.status == DownloadJobStatus::Paused)
    );
    manager
        .reorder(vec![a.id.clone(), c.id.clone()])
        .await
        .expect("reorder paused jobs");
    assert!(!started(&fixture, &third));
    finish(manager.suspend_all()).await.expect("shutdown");
    drop(manager);
    fixture.reopen_database().await;
    let recovered = fixture.manager_with(queue_executor, reqwest::Client::new());
    assert_eq!(
        recovered
            .recover_pending()
            .await
            .expect("keep explicit pauses"),
        0
    );
    recovered.resume_all().await.expect("resume queue");
    wait_status(&recovered, &b.id, DownloadJobStatus::Running).await;
    assert_eq!(
        recovered
            .stored_job(&a.id)
            .await
            .expect("waiting first")
            .status,
        DownloadJobStatus::Queued
    );
    assert!(!started(&fixture, &third));
    signal(&fixture, &second, "finish");
    finish(recovered.wait(&b.id))
        .await
        .expect("second completed");
    wait_status(&recovered, &a.id, DownloadJobStatus::Running).await;
    assert!(!started(&fixture, &third));
    signal(&fixture, &first, "finish");
    finish(recovered.wait(&a.id))
        .await
        .expect("first completed");
    wait_until(|| started(&fixture, &third)).await;
    signal(&fixture, &third, "finish");
    finish(recovered.wait(&c.id))
        .await
        .expect("third completed");
    finish(recovered.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn reordered_waiting_jobs_survive_restart_and_execute_in_the_new_order() {
    let mut fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    for invalid in [
        vec![c.id.clone(), c.id.clone()],
        vec![a.id.clone(), c.id.clone()],
        vec![b.id.clone()],
        vec!["missing".into(), c.id.clone()],
    ] {
        assert!(matches!(
            manager.reorder(invalid).await,
            Err(CoreError::DownloadQueueChanged)
        ));
    }
    let reordered = manager
        .reorder(vec![c.id.clone(), b.id.clone()])
        .await
        .expect("reorder");
    assert_eq!(
        reordered
            .iter()
            .filter(|job| job.status == DownloadJobStatus::Queued)
            .map(|job| job.id.clone())
            .collect::<Vec<_>>(),
        vec![c.id.clone(), b.id.clone()]
    );
    assert!(!started(&fixture, &second));
    assert!(!started(&fixture, &third));
    finish(manager.suspend_all()).await.expect("shutdown");
    drop(manager);
    fixture.reopen_database().await;
    let recovered = fixture.manager_with(queue_executor, reqwest::Client::new());
    assert_eq!(recovered.recover_pending().await.expect("restore"), 3);
    wait_status(&recovered, &a.id, DownloadJobStatus::Running).await;
    signal(&fixture, &first, "finish");
    finish(recovered.wait(&a.id))
        .await
        .expect("first completed");
    wait_until(|| started(&fixture, &third)).await;
    assert!(!started(&fixture, &second));
    signal(&fixture, &third, "finish");
    finish(recovered.wait(&c.id))
        .await
        .expect("third completed");
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(recovered.wait(&b.id))
        .await
        .expect("second completed");
    finish(recovered.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn a_failed_reorder_rolls_back_database_positions_and_preserves_the_live_queue() {
    use sqlx::Connection;
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    let mut database = crate::open_db(&fixture.settings).await.expect("database");
    sqlx::query("CREATE TRIGGER reject_reorder BEFORE UPDATE OF queue_order ON download_jobs WHEN NEW.id = (SELECT id FROM download_jobs ORDER BY id LIMIT 1 OFFSET 1) AND NEW.queue_order != OLD.queue_order BEGIN SELECT RAISE(ABORT, 'fixture reorder failure'); END").execute(&mut database).await.expect("failure trigger");
    assert!(
        manager
            .reorder(vec![c.id.clone(), b.id.clone()])
            .await
            .is_err()
    );
    let stored = manager.stored_jobs().await.expect("stored queue");
    assert_eq!(
        stored
            .iter()
            .filter(|job| job.status == DownloadJobStatus::Queued)
            .map(|job| job.id.clone())
            .collect::<Vec<_>>(),
        vec![b.id.clone(), c.id.clone()]
    );
    sqlx::query("DROP TRIGGER reject_reorder")
        .execute(&mut database)
        .await
        .expect("remove trigger");
    database.close().await.expect("close database");
    signal(&fixture, &first, "finish");
    finish(manager.wait(&a.id)).await.expect("first completed");
    wait_until(|| started(&fixture, &second)).await;
    assert!(!started(&fixture, &third));
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second completed");
    wait_until(|| started(&fixture, &third)).await;
    signal(&fixture, &third, "finish");
    finish(manager.wait(&c.id)).await.expect("third completed");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn pausing_all_blocks_the_queue_and_resuming_all_preserves_fifo() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    finish(manager.pause_all()).await.expect("pause all");
    assert!(manager.telemetry().paused);
    wait_status(&manager, &a.id, DownloadJobStatus::Paused).await;
    wait_status(&manager, &b.id, DownloadJobStatus::Paused).await;
    assert!(!started(&fixture, &second));
    finish(manager.resume_all()).await.expect("resume all");
    assert!(!manager.telemetry().paused);
    wait_status(&manager, &a.id, DownloadJobStatus::Running).await;
    assert!(!started(&fixture, &second));
    signal(&fixture, &first, "finish");
    finish(manager.wait(&a.id)).await.expect("first finished");
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

fn preparing_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let folder = job.instance_folder.ok_or(CoreError::InstancePreparing)?;
        context.stage(DownloadPhase::Extracting).await?;
        let root = runtime.game.root();
        std::fs::create_dir_all(root)?;
        std::fs::write(root.join(format!("{folder}.started")), b"started")?;
        while !root.join(format!("{folder}.finish")).exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        std::fs::write(root.join(format!("{folder}.extracted")), b"extracted")?;
        context.checkpoint().await?;
        std::fs::write(root.join(format!("{folder}.continued")), b"continued")?;
        Ok(())
    })
}

#[tokio::test]
async fn pause_all_finishes_extraction_before_parking_at_a_safe_checkpoint() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(preparing_executor, reqwest::Client::new());
    let first = fixture.instance("Extracting").await;
    let second = fixture.instance("Waiting").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    finish(manager.pause_all()).await.expect("request pause");
    assert_eq!(
        manager.stored_job(&a.id).await.expect("extracting").status,
        DownloadJobStatus::Running
    );
    assert!(!started(&fixture, &second));
    signal(&fixture, &first, "finish");
    wait_status(&manager, &a.id, DownloadJobStatus::Paused).await;
    assert!(
        fixture
            .game
            .root()
            .join(format!("{}.extracted", first.folder))
            .exists()
    );
    assert!(
        !fixture
            .game
            .root()
            .join(format!("{}.continued", first.folder))
            .exists()
    );
    assert!(!started(&fixture, &second));
    finish(manager.resume_all()).await.expect("resume");
    finish(manager.wait(&a.id)).await.expect("first finished");
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

fn signal(fixture: &Fixture, entry: &InstanceEntry, action: &str) {
    std::fs::write(
        fixture
            .game
            .root()
            .join(format!("{}.{action}", entry.folder)),
        b"ready",
    )
    .expect("signal worker");
}

async fn wait_status(manager: &DownloadManager, id: &str, status: DownloadJobStatus) {
    finish(async {
        loop {
            if manager.stored_job(id).await.expect("job status").status == status {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
}

#[tokio::test]
async fn cancelling_queued_and_running_installations_advances_to_the_next_job() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    assert_eq!(
        finish(manager.cancel(&b.id))
            .await
            .expect("cancel waiting")
            .status,
        DownloadJobStatus::Cancelled
    );
    assert!(!started(&fixture, &second));
    assert!(!started(&fixture, &third));
    assert_eq!(
        finish(manager.cancel(&a.id))
            .await
            .expect("cancel active")
            .status,
        DownloadJobStatus::Cancelled
    );
    wait_until(|| started(&fixture, &third)).await;
    signal(&fixture, &third, "finish");
    finish(manager.wait(&c.id)).await.expect("third finished");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn immediate_pause_and_resume_never_strands_an_installation_outside_the_queue() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let entry = fixture.instance("Quick controls").await;
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    wait_until(|| started(&fixture, &entry)).await;
    for _ in 0..5 {
        manager.pause(&job.id).await.expect("pause");
        manager.resume(&job.id).await.expect("resume immediately");
        wait_status(&manager, &job.id, DownloadJobStatus::Running).await;
    }
    signal(&fixture, &entry, "finish");
    assert_eq!(
        finish(manager.wait(&job.id))
            .await
            .expect("finished")
            .status,
        DownloadJobStatus::Completed
    );
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn a_failed_installation_releases_the_queue_without_being_retried() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    signal(&fixture, &first, "fail");
    let failed = finish(manager.wait(&a.id)).await.expect("failed snapshot");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    assert!(
        failed
            .error
            .as_deref()
            .is_some_and(|error| error.contains("fixture installation failed"))
    );
    wait_until(|| started(&fixture, &second)).await;
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    assert_eq!(manager.recover_pending().await.expect("recover"), 0);
    assert_eq!(
        manager
            .stored_job(&a.id)
            .await
            .expect("failed remains")
            .status,
        DownloadJobStatus::Failed
    );
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn restarting_an_installation_as_repair_joins_the_same_fifo() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let repair = finish(manager.start(&first.folder, DownloadJobOperation::Repair))
        .await
        .expect("repair");
    assert_eq!(repair.id, a.id);
    assert_eq!(repair.operation, DownloadJobOperation::Repair);
    wait_status(&manager, &b.id, DownloadJobStatus::Running).await;
    assert_eq!(
        manager
            .stored_job(&repair.id)
            .await
            .expect("repair waiting")
            .status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    wait_status(&manager, &repair.id, DownloadJobStatus::Running).await;
    signal(&fixture, &first, "finish");
    assert_eq!(
        finish(manager.wait(&repair.id))
            .await
            .expect("repair finished")
            .status,
        DownloadJobStatus::Completed
    );
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn restart_automatically_preserves_requeued_order_and_explicit_pauses() {
    let mut fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let paused = fixture.instance("Paused").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    let p = manager
        .start(&paused.folder, DownloadJobOperation::Install)
        .await
        .expect("paused");
    manager.pause(&p.id).await.expect("pause waiting");
    manager.pause(&a.id).await.expect("pause active");
    wait_until(|| started(&fixture, &second)).await;
    manager.resume(&a.id).await.expect("requeue first at tail");
    finish(manager.suspend_all()).await.expect("shutdown");
    drop(manager);
    fixture.reopen_database().await;
    let recovered = fixture.manager_with(queue_executor, reqwest::Client::new());
    assert_eq!(recovered.recover_pending().await.expect("restore queue"), 3);
    assert_eq!(
        recovered
            .recover_pending()
            .await
            .expect("idempotent recovery"),
        0
    );
    wait_status(&recovered, &b.id, DownloadJobStatus::Running).await;
    assert_eq!(
        recovered
            .stored_job(&c.id)
            .await
            .expect("third queued")
            .status,
        DownloadJobStatus::Queued
    );
    assert_eq!(
        recovered
            .stored_job(&a.id)
            .await
            .expect("first at tail")
            .status,
        DownloadJobStatus::Queued
    );
    assert_eq!(
        recovered
            .stored_job(&p.id)
            .await
            .expect("explicit pause")
            .status,
        DownloadJobStatus::Paused
    );
    assert!(!started(&fixture, &paused));
    signal(&fixture, &second, "finish");
    finish(recovered.wait(&b.id))
        .await
        .expect("second finished");
    wait_status(&recovered, &c.id, DownloadJobStatus::Running).await;
    assert_eq!(
        recovered
            .stored_job(&a.id)
            .await
            .expect("first still at tail")
            .status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &third, "finish");
    finish(recovered.wait(&c.id)).await.expect("third finished");
    wait_status(&recovered, &a.id, DownloadJobStatus::Running).await;
    signal(&fixture, &first, "finish");
    finish(recovered.wait(&a.id)).await.expect("first finished");
    recovered
        .cancel(&p.id)
        .await
        .expect("cancel explicit pause");
    finish(recovered.suspend_all())
        .await
        .expect("shutdown recovered");
    fixture.close().await;
}

#[tokio::test]
async fn instance_installations_wait_in_fifo_order() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !started(&fixture, &second),
        "queued installation must not execute"
    );
    assert!(!started(&fixture, &third));
    assert_eq!(
        manager.stored_job(&b.id).await.expect("queued").status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &first, "finish");
    assert_eq!(
        finish(manager.wait(&a.id))
            .await
            .expect("first finished")
            .status,
        DownloadJobStatus::Completed
    );
    wait_until(|| started(&fixture, &second)).await;
    assert!(!started(&fixture, &third));
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    wait_until(|| started(&fixture, &third)).await;
    signal(&fixture, &third, "finish");
    finish(manager.wait(&c.id)).await.expect("third finished");
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}

#[tokio::test]
async fn pausing_releases_the_queue_and_resume_joins_the_tail() {
    let fixture = Fixture::new("unused").await;
    let manager = fixture.manager_with(queue_executor, reqwest::Client::new());
    let first = fixture.instance("First").await;
    let second = fixture.instance("Second").await;
    let third = fixture.instance("Third").await;
    let a = manager
        .start(&first.folder, DownloadJobOperation::Install)
        .await
        .expect("first");
    wait_until(|| started(&fixture, &first)).await;
    let b = manager
        .start(&second.folder, DownloadJobOperation::Install)
        .await
        .expect("second");
    let c = manager
        .start(&third.folder, DownloadJobOperation::Install)
        .await
        .expect("third");
    assert_eq!(
        finish(manager.pause(&a.id)).await.expect("pause").status,
        DownloadJobStatus::Paused
    );
    wait_until(|| started(&fixture, &second)).await;
    assert_eq!(
        manager.resume(&a.id).await.expect("resume").status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &first, "finish");
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(
        manager.stored_job(&a.id).await.expect("waiting").status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &second, "finish");
    finish(manager.wait(&b.id)).await.expect("second finished");
    wait_until(|| started(&fixture, &third)).await;
    assert_eq!(
        manager.stored_job(&a.id).await.expect("tail").status,
        DownloadJobStatus::Queued
    );
    signal(&fixture, &third, "finish");
    finish(manager.wait(&c.id)).await.expect("third finished");
    assert_eq!(
        finish(manager.wait(&a.id))
            .await
            .expect("resumed finished")
            .status,
        DownloadJobStatus::Completed
    );
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
}
