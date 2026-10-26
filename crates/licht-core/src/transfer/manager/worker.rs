use super::{ActiveJob, DownloadManager};
use crate::{
    CoreError, DownloadControl, DownloadJobSnapshot, DownloadJobStatus, DownloadState,
    InstallationContext, Result,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, mpsc, watch};

impl DownloadManager {
    pub(super) async fn spawn(self: &Arc<Self>, snapshot: DownloadJobSnapshot) {
        let (completion, _) = watch::channel(None);
        let (stopped, _) = watch::channel(false);
        let (enabled, _) = watch::channel(true);
        let slot = Arc::new(ActiveJob {
            control: DownloadControl::with_counters(Arc::clone(&self.counters)),
            snapshot: Mutex::new(snapshot.clone()),
            completion,
            stopped,
            worker: Mutex::new(None),
            suspending: AtomicBool::new(false),
            restarting: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            enabled,
        });
        if self.all_paused.load(Ordering::SeqCst) {
            slot.control.request_pause();
        }
        if snapshot.instance_folder.is_some() {
            self.installations
                .lock()
                .await
                .waiting
                .push_back(snapshot.id.clone());
            self.queue_changed.notify_waiters();
        }
        self.active
            .lock()
            .await
            .insert(snapshot.id.clone(), Arc::clone(&slot));
        self.emit(snapshot.clone());
        let manager = Arc::clone(self);
        let worker_slot = Arc::clone(&slot);
        let worker = tokio::spawn(async move { manager.drive(snapshot.id, worker_slot).await });
        let abort = worker.abort_handle();
        *slot.worker.lock().await = Some(abort.clone());
        if slot.suspending.load(Ordering::SeqCst) {
            abort.abort();
        }
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let outcome = worker.await;
            let id = slot.snapshot.lock().await.id.clone();
            manager.release_turn(&id).await;
            let failure = match outcome {
                Ok(Ok(())) => {
                    slot.stopped.send_replace(true);
                    return;
                }
                Ok(Err(error)) => error.to_string(),
                Err(_) => "download worker stopped unexpectedly".to_string(),
            };
            if slot.suspending.load(Ordering::SeqCst) || slot.restarting.load(Ordering::SeqCst) {
                let id = slot.snapshot.lock().await.id.clone();
                let message = if slot.restarting.load(Ordering::SeqCst) {
                    "download restarted as a full repair"
                } else {
                    "download suspended for shutdown"
                };
                slot.completion.send_replace(Some(Err(message.to_string())));
                manager.active.lock().await.remove(&id);
                slot.stopped.send_replace(true);
                return;
            }
            manager.fail(Arc::clone(&slot), failure).await;
            slot.stopped.send_replace(true);
        });
    }

    async fn drive(self: &Arc<Self>, id: String, slot: Arc<ActiveJob>) -> Result<()> {
        let result = self.execute_job(&id, &slot).await;
        self.finish_job(&id, &slot, result).await
    }

    async fn execute_job(self: &Arc<Self>, id: &str, slot: &Arc<ActiveJob>) -> Result<()> {
        let initial = slot.snapshot.lock().await.clone();
        let instance_job = initial.instance_folder.is_some();
        if instance_job {
            self.wait_turn(id, slot).await?;
        }
        let (events, mut incoming) = mpsc::channel(4096);
        let context = InstallationContext::for_job(
            self.runtime.settings.clone(),
            self.runtime.game.clone(),
            id.to_string(),
            slot.control.clone(),
            events,
            self.runtime.database.clone(),
        );
        let operation = (self.executor)(self.runtime.clone(), initial, context);
        tokio::pin!(operation);
        let mut last_emit = Instant::now();
        let mut last_save = Instant::now();
        let mut observing = true;
        let mut ticks = tokio::time::interval(Duration::from_millis(100));
        let mut enabled = slot.enabled.subscribe();
        let result = loop {
            let owns_turn =
                !instance_job || self.installations.lock().await.current.as_deref() == Some(id);
            if instance_job && (!*enabled.borrow() || !owns_turn) {
                // Keep polling transfers until Pause has closed their files, then
                // park this pipeline without losing its completion subscribers.
                loop {
                    tokio::select! {
                        biased;
                        event = incoming.recv(), if observing => {
                            if let Some(event) = event {
                                self.apply_event(slot, event, &mut last_emit, &mut last_save).await?;
                            } else { observing = false; }
                        },
                        result = &mut operation => return result,
                        idle = slot.control.wait_for_idle() => { idle?; break; },
                    }
                }
                // The final transfer poll can forward progress and become idle together.
                while let Ok(event) = incoming.try_recv() {
                    self.apply_event(slot, event, &mut last_emit, &mut last_save)
                        .await?;
                }
                {
                    let mut snapshot = slot.snapshot.lock().await;
                    if snapshot.status == DownloadJobStatus::Paused
                        && !self.closing.load(Ordering::SeqCst)
                    {
                        self.persist(&mut snapshot).await?;
                        self.emit(snapshot.clone());
                        slot.dirty.store(false, Ordering::SeqCst);
                    }
                }
                self.yield_turn(id).await;
                self.wait_turn(id, slot).await?;
            }
            tokio::select! {
                biased;
                changed = enabled.changed(), if instance_job => {
                    changed.map_err(|_| CoreError::DownloadCancelled)?;
                },
                event = incoming.recv(), if observing => {
                    if let Some(event) = event {
                        self.apply_event(slot, event, &mut last_emit, &mut last_save).await?;
                    } else {
                        observing = false;
                    }
                }
                result = &mut operation => break result,
                _ = ticks.tick() => {
                    self.flush_progress(slot, &mut last_emit, &mut last_save).await?;
                }
            }
        };
        while let Ok(event) = incoming.try_recv() {
            self.apply_event(slot, event, &mut last_emit, &mut last_save)
                .await?;
        }
        result
    }

    async fn finish_job(&self, id: &str, slot: &ActiveJob, result: Result<()>) -> Result<()> {
        let mut snapshot = slot.snapshot.lock().await;
        if slot.restarting.load(Ordering::SeqCst) {
            slot.completion
                .send_replace(Some(Err("download restarted as a full repair".to_string())));
            self.active.lock().await.remove(id);
            return Ok(());
        }
        let result = if slot.control.state() == DownloadState::Cancelled {
            Err(CoreError::DownloadCancelled)
        } else {
            result
        };
        if matches!(result, Err(CoreError::DownloadCancelled)) {
            self.cleanup_artifacts(id).await?;
        }
        if result.is_ok()
            && let Some(folder) = &snapshot.instance_folder
        {
            crate::modpacks::workspace::cleanup(&self.runtime.game.instance_dir(folder)?)?;
        }
        snapshot.status = match &result {
            Ok(()) => DownloadJobStatus::Completed,
            Err(CoreError::DownloadCancelled)
                if snapshot.status == DownloadJobStatus::Cancelling =>
            {
                DownloadJobStatus::Cancelling
            }
            Err(CoreError::DownloadCancelled) => DownloadJobStatus::Cancelled,
            Err(_) => DownloadJobStatus::Failed,
        };
        snapshot.error = result
            .err()
            .filter(|error| !matches!(error, CoreError::DownloadCancelled))
            .map(|error| error.to_string());
        self.persist(&mut snapshot).await?;
        self.emit(snapshot.clone());
        if snapshot.status == DownloadJobStatus::Cancelling {
            // The cancel command still needs to remove the instance directory.
            return Ok(());
        }
        slot.completion.send_replace(Some(Ok(snapshot.clone())));
        drop(snapshot);
        self.active.lock().await.remove(id);
        Ok(())
    }

    async fn fail(&self, slot: Arc<ActiveJob>, detail: String) {
        slot.control.cancel();
        let mut snapshot = slot.snapshot.lock().await;
        if snapshot.status != DownloadJobStatus::Cancelling {
            snapshot.status = DownloadJobStatus::Failed;
        }
        snapshot.error = Some(detail.clone());
        let outcome = match self.persist(&mut snapshot).await {
            Ok(()) => {
                self.emit(snapshot.clone());
                if snapshot.status == DownloadJobStatus::Cancelling {
                    Err(detail.clone())
                } else {
                    Ok(snapshot.clone())
                }
            }
            Err(error) => Err(error.to_string()),
        };
        slot.completion.send_replace(Some(outcome));
        let id = snapshot.id.clone();
        drop(snapshot);
        self.active.lock().await.remove(&id);
    }
}
