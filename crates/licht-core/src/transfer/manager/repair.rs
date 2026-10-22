use std::sync::atomic::Ordering;

use super::DownloadManager;
use crate::{
    CoreError, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus, Result, load_instance,
};

impl DownloadManager {
    // The caller holds `starts` until the previous worker is stopped and its replacement is spawned.
    pub(super) async fn restart_repair(
        self: &std::sync::Arc<Self>,
        job: DownloadJobSnapshot,
    ) -> Result<DownloadJobSnapshot> {
        if job.status == DownloadJobStatus::Cancelling {
            return Err(CoreError::DownloadJobTransition);
        }
        let folder = job
            .instance_folder
            .as_deref()
            .ok_or(CoreError::InstancePreparing)?;
        if let Some(slot) = self.active_job(&job.id).await {
            let finalization = slot.snapshot.lock().await;
            slot.restarting.store(true, Ordering::SeqCst);
            // Let bounded writes and processor shutdown finish before replacing the worker.
            slot.control.cancel();
            drop(finalization);
            let mut stopped = slot.stopped.subscribe();
            stopped.wait_for(|stopped| *stopped).await.map_err(|_| {
                CoreError::DownloadJobExecution {
                    detail: "download worker closed while restarting Repair".into(),
                }
            })?;
            self.active.lock().await.remove(&job.id);
        }
        self.accept_work()?;
        let mut latest = self.stored_job(&job.id).await?;
        if matches!(
            latest.status,
            DownloadJobStatus::Running | DownloadJobStatus::Queued
        ) {
            // Keep Resume available if the following Repair transaction fails.
            latest.status = DownloadJobStatus::Interrupted;
            self.persist(&mut latest).await?;
            self.emit(latest.clone());
        }
        let profile = load_instance(
            &self.runtime.database,
            &self.runtime.game.instance_dir(folder)?,
        )
        .await?;
        let restarted = if latest.status == DownloadJobStatus::Completed {
            // The previous worker may have finished while Repair was waiting for it to stop.
            self.runtime
                .database
                .create_pending_download_job(
                    &self.runtime.game,
                    folder,
                    profile.version_id.as_deref(),
                    DownloadJobOperation::Repair,
                )
                .await?
        } else {
            self.runtime
                .database
                .restart_download_as_repair(
                    &self.runtime.game,
                    &job.id,
                    profile.version_id.as_deref(),
                )
                .await?
        };
        self.spawn(restarted.clone()).await;
        Ok(restarted)
    }
}
