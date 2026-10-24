use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::{ActiveJob, DownloadManager};
use crate::{CoreError, DownloadJobStatus, DownloadState, Result, load_instance};

impl DownloadManager {
    pub(super) async fn wait_turn(&self, id: &str, slot: &ActiveJob) -> Result<()> {
        let mut enabled = slot.enabled.subscribe();
        loop {
            let changed = self.queue_changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if slot.control.state() == DownloadState::Cancelled {
                return Err(CoreError::DownloadCancelled);
            }
            let mut queue = self.installations.lock().await;
            if *enabled.borrow()
                && !self.closing.load(Ordering::SeqCst)
                && queue.current.is_none()
                && queue.waiting.front().is_some_and(|first| first == id)
            {
                queue.waiting.pop_front();
                queue.current = Some(id.to_string());
                drop(queue);
                let mut snapshot = slot.snapshot.lock().await;
                if self.installations.lock().await.current.as_deref() != Some(id) {
                    continue;
                }
                if !*enabled.borrow() || slot.control.state() == DownloadState::Cancelled {
                    drop(snapshot);
                    self.yield_turn(id).await;
                    continue;
                }
                snapshot.status = DownloadJobStatus::Running;
                self.persist(&mut snapshot).await?;
                slot.control.resume();
                self.emit(snapshot.clone());
                return Ok(());
            }
            drop(queue);
            tokio::select! {
                _ = &mut changed => {},
                result = enabled.changed() => {
                    result.map_err(|_| CoreError::DownloadCancelled)?;
                },
                _ = slot.control.cancelled() => return Err(CoreError::DownloadCancelled),
            }
        }
    }

    pub(super) async fn release_turn(&self, id: &str) {
        let mut queue = self.installations.lock().await;
        queue.waiting.retain(|pending| pending != id);
        if queue.current.as_deref() == Some(id) {
            queue.current = None;
        }
        drop(queue);
        self.queue_changed.notify_waiters();
    }

    pub(super) async fn yield_turn(&self, id: &str) {
        let mut queue = self.installations.lock().await;
        if queue.current.as_deref() == Some(id) {
            queue.current = None;
        }
        drop(queue);
        self.queue_changed.notify_waiters();
    }

    /// Restore pending instance installations in their persisted FIFO order.
    pub async fn recover_pending(self: &Arc<Self>) -> Result<usize> {
        let _starts = self.starts.lock().await;
        self.accept_work()?;
        let mut recovered = 0;
        for mut job in self.stored_jobs().await? {
            let Some(folder) = &job.instance_folder else {
                continue;
            };
            if !matches!(
                job.status,
                DownloadJobStatus::Interrupted
                    | DownloadJobStatus::Queued
                    | DownloadJobStatus::Running
            ) || self.active_job(&job.id).await.is_some()
            {
                continue;
            }
            if let Err(error) = load_instance(
                &self.runtime.database,
                &self.runtime.game.instance_dir(folder)?,
            )
            .await
            {
                job.status = DownloadJobStatus::Failed;
                job.error = Some(error.to_string());
                self.persist(&mut job).await?;
                self.emit(job);
                continue;
            }
            if job.status == DownloadJobStatus::Running {
                job.status = DownloadJobStatus::Interrupted;
                self.persist(&mut job).await?;
            }
            job.status = DownloadJobStatus::Queued;
            job.error = None;
            self.persist(&mut job).await?;
            self.spawn(job).await;
            recovered += 1;
        }
        Ok(recovered)
    }
}
