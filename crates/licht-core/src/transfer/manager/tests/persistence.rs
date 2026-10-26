use std::future::Future;
use std::task::Poll;
use std::time::Instant;

use sqlx::Connection;

use super::*;
use crate::transfer::installation::InstallationEvent;

#[tokio::test]
async fn shutdown_pause_acknowledgement_does_not_create_an_explicit_user_pause() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let entry = fixture.instance("Shutdown acknowledgement").await;
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    let slot = manager.active_job(&job.id).await.expect("active job");
    let starts = manager.starts.lock().await;
    let mut shutdown = Box::pin(manager.suspend_all());
    std::future::poll_fn(|context| {
        assert!(shutdown.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    // Force the worker's pause acknowledgement into the shutdown window.
    slot.control.pause();
    finish(manager.acknowledge_pause(&slot))
        .await
        .expect("acknowledge shutdown pause");
    drop(starts);
    finish(shutdown).await.expect("shutdown");
    let stored = manager.stored_job(&job.id).await.expect("stored job");
    drop(tail);
    fixture.close().await;
    assert_eq!(stored.status, DownloadJobStatus::Interrupted);
}

fn late_progress_executor(
    runtime: DownloadRuntime,
    _job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        context.stage(DownloadPhase::Downloading).await?;
        let activity = context.control.begin_transfer()?;
        std::fs::create_dir_all(runtime.game.root())?;
        std::fs::write(runtime.game.root().join("transfer.started"), b"started")?;
        let interruption = context.control.interrupted().await;
        if !matches!(interruption, CoreError::DownloadPaused) {
            return Err(interruption);
        }
        context
            .send(crate::DownloadProgress {
                finished: 0,
                failed: 0,
                total: 1,
                bytes_done: 5,
                bytes_total: 11,
            })
            .await?;
        drop(activity);
        context.checkpoint().await
    })
}

#[tokio::test]
async fn progress_emitted_while_pausing_is_persisted_before_the_worker_parks() {
    let fixture = Fixture::new("unused").await;
    let entry = fixture.instance("Late pause progress").await;
    let manager = fixture.manager_with(late_progress_executor, reqwest::Client::new());
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    wait_until(|| fixture.game.root().join("transfer.started").exists()).await;
    let paused = finish(manager.pause(&job.id)).await.expect("pause");
    assert_eq!(paused.status, DownloadJobStatus::Paused);
    let persisted = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let stored = manager.stored_job(&job.id).await.expect("stored progress");
            if stored.progress.bytes_done == 5 {
                return stored;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    finish(manager.suspend_all()).await.expect("shutdown");
    fixture.close().await;
    let stored = persisted.expect("paused progress must be persisted without resuming");
    assert_eq!(stored.status, DownloadJobStatus::Paused);
}

#[tokio::test]
async fn a_contended_progress_write_yields_and_cannot_overwrite_shutdown_state() {
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let entry = fixture.instance("Contended progress").await;
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    let slot = manager.active_job(&job.id).await.expect("active job");
    let mut events = manager.subscribe();
    let mut writer = crate::open_db(&fixture.settings).await.expect("writer");
    let transaction = writer
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("writer lock");
    let mut last_emit = Instant::now();
    let mut last_save = Instant::now() - Duration::from_secs(2);
    let progress = crate::DownloadProgress {
        finished: 0,
        failed: 0,
        total: 1,
        bytes_done: 5,
        bytes_total: 11,
    };
    let mut save = Box::pin(manager.apply_event(
        &slot,
        InstallationEvent::Progress(progress),
        &mut last_emit,
        &mut last_save,
    ));
    std::future::poll_fn(|context| {
        assert!(
            save.as_mut().poll(context).is_pending(),
            "a blocked write must yield to Tokio"
        );
        Poll::Ready(())
    })
    .await;
    let mut shutdown = Box::pin(manager.suspend_all());
    std::future::poll_fn(|context| {
        assert!(shutdown.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    tokio::task::yield_now().await;
    transaction.rollback().await.expect("release writer");
    let (saved, stopped) = finish(async { tokio::join!(save, shutdown) }).await;
    saved.expect("progress persisted");
    stopped.expect("shutdown persisted");
    writer.close().await.expect("close writer");
    let stored = fixture
        .database()
        .get_download_job(&fixture.game, &job.id)
        .await
        .expect("persisted shutdown");
    assert_eq!(stored.status, DownloadJobStatus::Interrupted);
    assert_eq!(stored.progress, progress);
    assert!(manager.active.lock().await.is_empty());
    let mut last_status = None;
    while let Ok(event) = events.try_recv() {
        last_status = Some(event.status);
    }
    assert_eq!(last_status, Some(DownloadJobStatus::Interrupted));
    drop(tail);
    fixture.close().await;
}
