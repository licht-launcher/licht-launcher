use super::{ActiveJob, DownloadManager};
use crate::transfer::installation::InstallationEvent;
use crate::{
    DownloadJobSnapshot, DownloadJobStatus, DownloadJobUpdate, DownloadProgress, DownloadState,
    Result,
};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl DownloadManager {
    pub(super) async fn apply_event(
        &self,
        slot: &ActiveJob,
        event: InstallationEvent,
        last_emit: &mut Instant,
        last_save: &mut Instant,
    ) -> Result<()> {
        let mut snapshot = slot.snapshot.lock().await;
        if slot.control.state() == DownloadState::Cancelled {
            return Ok(());
        }
        let structural = match event {
            InstallationEvent::Stage(component, phase) => {
                snapshot.component = Some(component);
                snapshot.phase = phase;
                snapshot.progress = empty_progress();
                true
            }
            InstallationEvent::Version(version) => {
                self.runtime
                    .database
                    .set_download_job_version(&self.runtime.game, &snapshot.id, &version)
                    .await?;
                snapshot.version_id = Some(version);
                true
            }
            InstallationEvent::Progress(progress) => {
                snapshot.progress = progress;
                false
            }
        };
        snapshot.status = if slot.control.state() == DownloadState::Paused {
            if *slot.enabled.borrow() {
                DownloadJobStatus::Queued
            } else {
                DownloadJobStatus::Paused
            }
        } else {
            DownloadJobStatus::Running
        };
        slot.dirty.store(true, Ordering::SeqCst);
        if structural || last_save.elapsed() >= Duration::from_secs(1) {
            self.persist(&mut snapshot).await?;
            *last_save = Instant::now();
        }
        if structural || last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit(snapshot.clone());
            slot.dirty.store(false, Ordering::SeqCst);
            *last_emit = Instant::now();
        }
        Ok(())
    }

    pub(super) async fn persist(&self, snapshot: &mut DownloadJobSnapshot) -> Result<()> {
        *snapshot = self
            .runtime
            .database
            .update_download_job(
                &self.runtime.game,
                &snapshot.id,
                DownloadJobUpdate {
                    status: snapshot.status,
                    component: snapshot.component,
                    phase: snapshot.phase,
                    progress: snapshot.progress,
                    error: snapshot.error.clone(),
                },
            )
            .await?;
        Ok(())
    }
    pub(super) fn emit(&self, snapshot: DownloadJobSnapshot) {
        let _optional_observers = self.events.send(snapshot);
    }

    pub(super) async fn flush_progress(
        &self,
        slot: &ActiveJob,
        last_emit: &mut Instant,
        last_save: &mut Instant,
    ) -> Result<()> {
        self.acknowledge_pause(slot).await?;
        if !slot.dirty.load(Ordering::SeqCst) || last_emit.elapsed() < Duration::from_millis(100) {
            return Ok(());
        }
        let mut snapshot = slot.snapshot.lock().await;
        if slot.control.state() == DownloadState::Cancelled {
            return Ok(());
        }
        if snapshot.status == DownloadJobStatus::Paused
            || last_save.elapsed() >= Duration::from_secs(1)
        {
            self.persist(&mut snapshot).await?;
            *last_save = Instant::now();
        }
        self.emit(snapshot.clone());
        slot.dirty.store(false, Ordering::SeqCst);
        *last_emit = Instant::now();
        Ok(())
    }
}

fn empty_progress() -> DownloadProgress {
    DownloadProgress {
        finished: 0,
        failed: 0,
        total: 0,
        bytes_done: 0,
        bytes_total: 0,
    }
}
