use super::DownloadManager;
use crate::{
    CoreError, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus, Result, load_instance,
};

impl DownloadManager {
    pub async fn cancel(&self, id: &str) -> Result<DownloadJobSnapshot> {
        let _starts = self.starts.lock().await;
        let job = if let Some(slot) = self.active_job(id).await {
            let mut snapshot = slot.snapshot.lock().await;
            let next = self.prepare_cancellation(snapshot.clone()).await?;
            *snapshot = next;
            if snapshot.status == DownloadJobStatus::Cancelling {
                slot.control.cancel();
            }
            snapshot.clone()
        } else {
            self.prepare_cancellation(self.stored_job(id).await?)
                .await?
        };
        if matches!(
            job.status,
            DownloadJobStatus::Completed | DownloadJobStatus::Cancelled
        ) {
            return Ok(job);
        }
        let result = async {
            let stopped = self.cancel_job(id).await?;
            self.finish_cancellation(stopped).await
        }
        .await;
        let result = match result {
            Err(error) => match self.record_cancellation_failure(id, &error).await {
                Ok(()) => Err(error),
                Err(record_error) => Err(CoreError::DownloadJobExecution {
                    detail: format!("{error}; failed to record cancellation error: {record_error}"),
                }),
            },
            success => success,
        };
        self.complete_cancellation(id, &result).await;
        result
    }

    async fn complete_cancellation(&self, id: &str, result: &Result<DownloadJobSnapshot>) {
        if let Some(slot) = self.active_job(id).await {
            slot.completion
                .send_replace(Some(result.as_ref().cloned().map_err(ToString::to_string)));
            self.active.lock().await.remove(id);
        }
    }

    pub(super) async fn prepare_cancellation(
        &self,
        mut job: DownloadJobSnapshot,
    ) -> Result<DownloadJobSnapshot> {
        if matches!(
            job.status,
            DownloadJobStatus::Completed
                | DownloadJobStatus::Cancelled
                | DownloadJobStatus::Cancelling
        ) {
            return Ok(job);
        }
        if job.operation == DownloadJobOperation::Install
            && let Some(folder) = &job.instance_folder
        {
            let profile = load_instance(&self.runtime.game.instance_dir(folder)?)?;
            if profile.installation.is_some_and(|installation| {
                installation.state != crate::InstancePreparationState::Ready
            }) {
                // Persist removal intent before a partial deletion can remove instance.json.
                job.status = DownloadJobStatus::Cancelling;
                job.error = None;
                self.persist(&mut job).await?;
                self.emit(job.clone());
            }
        }
        Ok(job)
    }

    async fn finish_cancellation(
        &self,
        mut job: DownloadJobSnapshot,
    ) -> Result<DownloadJobSnapshot> {
        if job.status != DownloadJobStatus::Cancelling {
            return Ok(job);
        }
        let folder = job
            .instance_folder
            .as_deref()
            .ok_or(CoreError::InstancePreparing)?;
        let directory = self.runtime.game.instance_dir(folder)?;
        if let Err(error) = tokio::fs::remove_dir_all(&directory).await
            && (error.kind() != std::io::ErrorKind::NotFound
                || tokio::fs::try_exists(&directory).await?)
        {
            return Err(error.into());
        }
        self.runtime
            .database
            .delete_modpack(&self.runtime.game.instances_dir()?, folder)
            .await?;
        job.status = DownloadJobStatus::Cancelled;
        job.error = None;
        self.persist(&mut job).await?;
        self.emit(job.clone());
        Ok(job)
    }

    async fn record_cancellation_failure(&self, id: &str, error: &CoreError) -> Result<()> {
        let mut job = self.stored_job(id).await?;
        if job.status == DownloadJobStatus::Cancelling {
            job.error = Some(error.to_string());
            self.persist(&mut job).await?;
            self.emit(job);
        }
        Ok(())
    }

    pub(super) async fn cancel_job(&self, id: &str) -> Result<DownloadJobSnapshot> {
        if let Some(slot) = self.active_job(id).await {
            let snapshot = slot.snapshot.lock().await;
            if matches!(
                snapshot.status,
                DownloadJobStatus::Completed | DownloadJobStatus::Cancelled
            ) {
                return Ok(snapshot.clone());
            }
            slot.control.cancel();
            drop(snapshot);
            let mut stopped = slot.stopped.subscribe();
            stopped.wait_for(|stopped| *stopped).await.map_err(|_| {
                CoreError::DownloadJobExecution {
                    detail: "download worker stopped without notification".into(),
                }
            })?;
            if let Some(Err(detail)) = slot.completion.borrow().as_ref() {
                return Err(CoreError::DownloadJobExecution {
                    detail: detail.clone(),
                });
            }
            let outcome = slot.snapshot.lock().await.clone();
            if outcome.status == DownloadJobStatus::Cancelling
                && let Some(detail) = outcome.error
            {
                return Err(CoreError::DownloadJobExecution { detail });
            }
            if outcome.status != DownloadJobStatus::Failed {
                return Ok(outcome);
            }
        }
        let mut snapshot = self.stored_job(id).await?;
        if matches!(
            snapshot.status,
            DownloadJobStatus::Completed | DownloadJobStatus::Cancelled
        ) {
            return Ok(snapshot);
        }
        self.cleanup_artifacts(id).await?;
        if snapshot.status != DownloadJobStatus::Cancelling {
            snapshot.status = DownloadJobStatus::Cancelled;
        }
        snapshot.error = None;
        self.persist(&mut snapshot).await?;
        self.emit(snapshot.clone());
        Ok(snapshot)
    }

    pub async fn cancel_instance(&self, folder: &str) -> Result<()> {
        let _starts = self.starts.lock().await;
        let jobs = self.stored_jobs().await?;
        for job in jobs
            .into_iter()
            .filter(|job| job.instance_folder.as_deref() == Some(folder))
        {
            let mut stopped = self.cancel_job(&job.id).await?;
            if stopped.status == DownloadJobStatus::Cancelling {
                // The explicit instance-delete command owns directory removal.
                stopped.status = DownloadJobStatus::Cancelled;
                self.persist(&mut stopped).await?;
                self.emit(stopped.clone());
                self.complete_cancellation(&job.id, &Ok(stopped)).await;
            }
        }
        Ok(())
    }
}
