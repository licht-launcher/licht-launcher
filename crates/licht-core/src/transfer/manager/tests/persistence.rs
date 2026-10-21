use std::future::Future;
use std::task::Poll;
use std::time::Instant;

use sqlx::Connection;

use super::*;
use crate::transfer::installation::InstallationEvent;

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
