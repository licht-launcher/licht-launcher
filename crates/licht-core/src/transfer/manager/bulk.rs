use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::{ActiveJob, DownloadManager};
use crate::{DownloadJobStatus, DownloadPhase, DownloadState, DownloadTelemetry, Result};

impl DownloadManager {
    pub fn telemetry(&self) -> DownloadTelemetry {
        self.counters
            .snapshot(self.all_paused.load(Ordering::SeqCst))
    }

    pub async fn pause_all(&self) -> Result<()> {
        let _starts = self.starts.lock().await;
        self.accept_work()?;
        // Block admission until the current owner's priority is saved.
        let mut queue = self.installations.lock().await;
        let jobs = self.stored_jobs().await?;
        let unfinished = |id: &String| {
            jobs.iter().any(|job| {
                &job.id == id
                    && matches!(
                        job.status,
                        DownloadJobStatus::Running
                            | DownloadJobStatus::Queued
                            | DownloadJobStatus::Paused
                    )
            })
        };
        let owner = queue
            .current
            .as_ref()
            .filter(|id| unfinished(id))
            .cloned()
            .or_else(|| {
                jobs.iter()
                    .find(|job| job.status == DownloadJobStatus::Running)
                    .map(|job| job.id.clone())
            })
            .or_else(|| {
                queue
                    .last_active
                    .as_ref()
                    .filter(|id| unfinished(id))
                    .cloned()
            })
            .or_else(|| {
                jobs.iter()
                    .find(|job| job.status == DownloadJobStatus::Paused)
                    .map(|job| job.id.clone())
            });
        if let Some(id) = owner {
            self.runtime
                .database
                .prioritize_download_job(&self.runtime.game, &id)
                .await?;
            queue.last_active = Some(id);
        }
        self.all_paused.store(true, Ordering::SeqCst);
        drop(queue);
        let slots: Vec<_> = self.active.lock().await.values().cloned().collect();
        for slot in slots {
            let snapshot = slot.snapshot.lock().await;
            match snapshot.status {
                DownloadJobStatus::Running if snapshot.phase != DownloadPhase::Downloading => {
                    // The pipeline acknowledges this at its next safe checkpoint.
                    slot.control.request_pause();
                }
                DownloadJobStatus::Running | DownloadJobStatus::Queued => {
                    slot.control.pause();
                }
                _ => continue,
            }
            drop(snapshot);
            self.acknowledge_pause(&slot).await?;
        }
        Ok(())
    }

    pub async fn resume_all(self: &Arc<Self>) -> Result<()> {
        let _starts = self.starts.lock().await;
        self.accept_work()?;
        let was_paused = self.all_paused.load(Ordering::SeqCst);
        self.admission_blocked.store(true, Ordering::SeqCst);
        let result: Result<()> = async {
            let jobs = self.stored_jobs().await?;
            let order: Vec<_> = jobs
                .iter()
                .filter(|job| {
                    matches!(
                        job.status,
                        DownloadJobStatus::Queued | DownloadJobStatus::Paused
                    )
                })
                .map(|job| job.id.clone())
                .collect();
            // Keep admission blocked until all paused jobs have rejoined the FIFO.
            for job in jobs {
                if job.status == DownloadJobStatus::Paused {
                    self.resume_locked(&job.id).await?;
                } else if let Some(slot) = self.active_job(&job.id).await {
                    slot.control.clear_pause_request();
                    slot.control.resume();
                }
            }
            self.restore_waiting_order(&order).await?;
            Ok(())
        }
        .await;
        self.all_paused
            .store(result.is_err() && was_paused, Ordering::SeqCst);
        self.admission_blocked.store(false, Ordering::SeqCst);
        self.queue_changed.notify_waiters();
        result
    }

    pub(super) async fn acknowledge_pause(&self, slot: &ActiveJob) -> Result<()> {
        if slot.control.state() != DownloadState::Paused {
            return Ok(());
        }
        let mut snapshot = slot.snapshot.lock().await;
        // Shutdown owns the Interrupted transition; it is not an explicit user pause.
        if self.closing.load(Ordering::SeqCst) || slot.control.state() != DownloadState::Paused {
            return Ok(());
        }
        if !matches!(
            snapshot.status,
            DownloadJobStatus::Running | DownloadJobStatus::Queued
        ) {
            return Ok(());
        }
        slot.enabled.send_replace(false);
        snapshot.status = DownloadJobStatus::Paused;
        self.persist(&mut snapshot).await?;
        self.emit(snapshot.clone());
        let id = snapshot.id.clone();
        drop(snapshot);
        slot.control.wait_for_idle().await?;
        self.release_turn(&id).await;
        Ok(())
    }
}
