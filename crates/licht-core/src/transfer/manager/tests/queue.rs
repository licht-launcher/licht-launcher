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
