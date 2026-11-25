use crate::{
    CoreError, DatabaseClient, DownloadControl, DownloadJobOperation, DownloadJobSnapshot,
    DownloadJobStatus, DownloadPhase, DownloadState, InstallationContext, InstanceEntry,
    InstanceSource, LaunchEnvironment, Result, SharedCache, create_pending_instance, global_launch,
    load_instance, load_settings,
};
use futures_util::future::BoxFuture;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::sync::{Mutex, Notify, broadcast, watch};

mod bulk;
mod cancellation;
mod progress;
mod queue;
mod repair;
mod worker;

#[derive(Clone)]
pub(crate) struct DownloadRuntime {
    pub settings: crate::Database,
    pub game: SharedCache,
    pub environment: LaunchEnvironment,
    pub client: reqwest::Client,
    pub database: DatabaseClient,
}
type Executor =
    fn(DownloadRuntime, DownloadJobSnapshot, InstallationContext) -> BoxFuture<'static, Result<()>>;
type Outcome = std::result::Result<DownloadJobSnapshot, String>;
struct ActiveJob {
    control: DownloadControl,
    snapshot: Mutex<DownloadJobSnapshot>,
    completion: watch::Sender<Option<Outcome>>,
    stopped: watch::Sender<bool>,
    worker: Mutex<Option<tokio::task::AbortHandle>>,
    suspending: AtomicBool,
    restarting: AtomicBool,
    dirty: AtomicBool,
    enabled: watch::Sender<bool>,
    /// The tenth of the current stage last logged as download progress.
    logged_progress: AtomicU8,
}
#[derive(Default)]
struct InstallationQueue {
    waiting: VecDeque<String>,
    current: Option<String>,
    last_active: Option<String>,
}
/// The target of `Download progress` events, which the launcher keeps out of its
/// log file.
pub const DOWNLOAD_PROGRESS_TARGET: &str = "licht_core::transfer::progress";

pub struct DownloadManager {
    runtime: DownloadRuntime,
    executor: Executor,
    starts: Mutex<()>,
    active: Mutex<HashMap<String, Arc<ActiveJob>>>,
    events: broadcast::Sender<DownloadJobSnapshot>,
    closing: AtomicBool,
    installations: Mutex<InstallationQueue>,
    queue_changed: Notify,
    all_paused: AtomicBool,
    admission_blocked: AtomicBool,
    counters: Arc<super::control::TransferCounters>,
}

/// What a change job moves an instance to.
enum Change<'a> {
    /// Another platform or game version, for an unlinked instance.
    Base(InstanceSource),
    /// Another version of a linked store pack.
    ModpackVersion(&'a str),
}

impl DownloadManager {
    pub fn new(
        settings: crate::Database,
        game: SharedCache,
        environment: LaunchEnvironment,
        client: reqwest::Client,
        database: DatabaseClient,
    ) -> Self {
        let (events, _) = broadcast::channel(512);
        Self {
            runtime: DownloadRuntime {
                settings,
                game,
                environment,
                client,
                database,
            },
            executor: super::pipeline::execute,
            starts: Mutex::new(()),
            active: Mutex::new(HashMap::new()),
            events,
            closing: AtomicBool::new(false),
            installations: Mutex::new(InstallationQueue::default()),
            queue_changed: Notify::new(),
            all_paused: AtomicBool::new(false),
            admission_blocked: AtomicBool::new(false),
            counters: Arc::new(super::control::TransferCounters::default()),
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<DownloadJobSnapshot> {
        self.events.subscribe()
    }
    pub async fn create(
        self: &Arc<Self>,
        name: &str,
        source: InstanceSource,
    ) -> Result<InstanceEntry> {
        self.accept_work()?;
        let settings = load_settings(&self.runtime.settings).await?;
        let root = self.runtime.game.instances_dir()?;
        let entry = create_pending_instance(
            &self.runtime.database,
            &root,
            name,
            source,
            &global_launch(&settings),
        )
        .await?;
        self.runtime
            .database
            .delete_modpack(&root, &entry.folder)
            .await?;
        self.start(&entry.folder, DownloadJobOperation::Install)
            .await?;
        Ok(entry)
    }
    /// Creates an instance from a modpack archive on disk (`.mrpack` or CurseForge zip).
    pub async fn create_from_modpack_file(
        self: &Arc<Self>,
        name: &str,
        archive: &std::path::Path,
    ) -> Result<InstanceEntry> {
        self.accept_work()?;
        let source = crate::modpacks::detect_modpack_source(archive)?;
        let settings = load_settings(&self.runtime.settings).await?;
        let root = self.runtime.game.instances_dir()?;
        let entry = create_pending_instance(
            &self.runtime.database,
            &root,
            name,
            InstanceSource::ModpackFile { source },
            &global_launch(&settings),
        )
        .await?;
        if let Err(error) =
            crate::modpacks::stage_modpack_archive(&root.join(&entry.folder), archive)
        {
            if let Err(cleanup) =
                crate::delete_instance(&self.runtime.database, &root, &entry.folder).await
            {
                tracing::warn!(error = %cleanup, "could not remove an instance that failed to stage its archive");
            }
            return Err(error);
        }
        self.runtime
            .database
            .delete_modpack(&root, &entry.folder)
            .await?;
        self.start(&entry.folder, DownloadJobOperation::Install)
            .await?;
        Ok(entry)
    }
    /// Reinstalls a ready instance's store modpack and waits for it. Only a store pack
    /// can be fetched again; an archive imported from disk is not kept after install.
    pub async fn reinstall(self: &Arc<Self>, folder: &str) -> Result<DownloadJobSnapshot> {
        let profile = load_instance(
            &self.runtime.database,
            &self.runtime.game.instance_dir(folder)?,
        )
        .await?;
        let installation = profile
            .installation
            .as_ref()
            .ok_or(CoreError::InstanceNotLinked)?;
        match installation.source {
            InstanceSource::Modpack { .. } => {}
            InstanceSource::ModpackFile { .. } => return Err(CoreError::ModpackFileReinstall),
            _ => return Err(CoreError::InstanceNotLinked),
        }
        if installation.state != crate::InstancePreparationState::Ready {
            return Err(CoreError::InstancePreparing);
        }
        self.ensure_not_running(folder)?;
        self.prepare(folder, DownloadJobOperation::Reinstall).await
    }

    /// Moves a ready, unlinked instance to `target` and waits for its loader profile
    /// and game files. Until the job finishes the instance waits in `Preparing`;
    /// cancelling the job puts the previous base back.
    pub async fn change_base(
        self: &Arc<Self>,
        folder: &str,
        target: InstanceSource,
    ) -> Result<DownloadJobSnapshot> {
        self.start_change(folder, Change::Base(target)).await
    }

    /// Moves a ready instance linked to a store pack to the pack's `version_id` and
    /// waits for it. Content the user added stays; cancelling the job puts the
    /// previous version back.
    pub async fn change_modpack_version(
        self: &Arc<Self>,
        folder: &str,
        version_id: &str,
    ) -> Result<DownloadJobSnapshot> {
        self.start_change(folder, Change::ModpackVersion(version_id))
            .await
    }

    /// Starts a change job once nothing else runs for the instance and its game is
    /// closed. The change is recorded on the instance under the start lock, and
    /// undone if the job cannot be created.
    async fn start_change(
        self: &Arc<Self>,
        folder: &str,
        change: Change<'_>,
    ) -> Result<DownloadJobSnapshot> {
        let starts = self.starts.lock().await;
        self.accept_work()?;
        if self
            .stored_jobs()
            .await?
            .iter()
            .any(|job| job.instance_folder.as_deref() == Some(folder))
        {
            return Err(CoreError::DownloadJobExists);
        }
        self.ensure_not_running(folder)?;
        let root = self.runtime.game.instances_dir()?;
        let database = &self.runtime.database;
        let (entry, operation) = match change {
            Change::Base(target) => (
                crate::begin_instance_base_change(database, &root, folder, target).await?,
                DownloadJobOperation::ChangeBase,
            ),
            Change::ModpackVersion(version_id) => (
                crate::begin_modpack_version_change(database, &root, folder, version_id).await?,
                DownloadJobOperation::ChangeVersion,
            ),
        };
        let job = match self
            .runtime
            .database
            .create_pending_download_job(
                &self.runtime.game,
                folder,
                entry.version_id.as_deref(),
                operation,
            )
            .await
        {
            Ok(job) => job,
            Err(error) => {
                // No job will finish or cancel the change, so undo it now.
                crate::instance::restore_instance_base(&self.runtime.database, &root, folder)
                    .await?;
                return Err(error);
            }
        };
        self.spawn(job.clone()).await;
        drop(starts);
        self.wait(&job.id).await
    }

    /// Unlinks a ready instance from its modpack. Holding the start lock while no job
    /// exists for the instance keeps a running reinstall or repair from writing the
    /// pack back after it is forgotten.
    pub async fn unlink(self: &Arc<Self>, folder: &str) -> Result<InstanceEntry> {
        let _starts = self.starts.lock().await;
        if self
            .stored_jobs()
            .await?
            .iter()
            .any(|job| job.instance_folder.as_deref() == Some(folder))
        {
            return Err(CoreError::DownloadJobExists);
        }
        crate::unlink_instance_modpack(
            &self.runtime.database,
            &self.runtime.game.instances_dir()?,
            folder,
        )
        .await
    }

    /// Changing a pack under a running game fails halfway on locked files.
    fn ensure_not_running(&self, folder: &str) -> Result<()> {
        let dir = self.runtime.game.instance_dir(folder)?;
        match crate::read_launch_lock(&dir)? {
            Some(lock) if crate::process_matches_lock(lock.pid, lock.start_time) => {
                Err(CoreError::InstanceRunning)
            }
            _ => Ok(()),
        }
    }

    pub async fn prepare(
        self: &Arc<Self>,
        folder: &str,
        operation: DownloadJobOperation,
    ) -> Result<DownloadJobSnapshot> {
        let job = self.start(folder, operation).await?;
        self.wait(&job.id).await
    }
    async fn start(
        self: &Arc<Self>,
        folder: &str,
        operation: DownloadJobOperation,
    ) -> Result<DownloadJobSnapshot> {
        let starts = self.starts.lock().await;
        self.accept_work()?;
        if let Some(job) = self
            .stored_jobs()
            .await?
            .into_iter()
            .find(|job| job.instance_folder.as_deref() == Some(folder))
        {
            // Checked under the lock, so another task started meanwhile is never
            // resumed in place of a reinstall, and a repair never replaces a base or
            // version change. Preparing the instance for play resumes a base change.
            if (operation == DownloadJobOperation::Reinstall
                && job.operation != DownloadJobOperation::Reinstall)
                || (operation == DownloadJobOperation::Repair
                    && matches!(
                        job.operation,
                        DownloadJobOperation::ChangeBase | DownloadJobOperation::ChangeVersion
                    ))
            {
                return Err(CoreError::DownloadJobExists);
            }
            if operation == DownloadJobOperation::Repair {
                return self.restart_repair(job).await;
            }
            drop(starts);
            return match job.status {
                DownloadJobStatus::Paused
                | DownloadJobStatus::Interrupted
                | DownloadJobStatus::Failed => self.resume(&job.id).await,
                _ => Ok(job),
            };
        }
        let profile = load_instance(
            &self.runtime.database,
            &self.runtime.game.instance_dir(folder)?,
        )
        .await?;
        let job = self
            .runtime
            .database
            .create_pending_download_job(
                &self.runtime.game,
                folder,
                profile.version_id.as_deref(),
                operation,
            )
            .await?;
        self.spawn(job.clone()).await;
        Ok(job)
    }
    pub async fn pause(&self, id: &str) -> Result<DownloadJobSnapshot> {
        let _starts = self.starts.lock().await;
        let Some(slot) = self.active_job(id).await else {
            let snapshot = self.stored_job(id).await?;
            return if matches!(
                snapshot.status,
                DownloadJobStatus::Completed
                    | DownloadJobStatus::Cancelled
                    | DownloadJobStatus::Failed
            ) {
                Ok(snapshot)
            } else {
                Err(CoreError::DownloadJobTransition)
            };
        };
        let mut snapshot = slot.snapshot.lock().await;
        if matches!(
            snapshot.status,
            DownloadJobStatus::Completed | DownloadJobStatus::Cancelled | DownloadJobStatus::Failed
        ) {
            return Ok(snapshot.clone());
        }
        if snapshot.status == DownloadJobStatus::Paused {
            drop(snapshot);
            slot.control.wait_for_idle().await?;
            return Ok(slot.snapshot.lock().await.clone());
        }
        if snapshot.status == DownloadJobStatus::Running
            && snapshot.phase != DownloadPhase::Downloading
        {
            // The phase may have advanced since the UI offered Pause.
            return Ok(snapshot.clone());
        }
        if !matches!(
            snapshot.status,
            DownloadJobStatus::Queued | DownloadJobStatus::Running
        ) {
            return Err(CoreError::DownloadJobTransition);
        }
        let was_running = snapshot.status == DownloadJobStatus::Running;
        slot.control.pause();
        slot.enabled.send_replace(false);
        snapshot.status = DownloadJobStatus::Paused;
        self.persist(&mut snapshot).await?;
        self.emit(snapshot.clone());
        drop(snapshot);
        tracing::info!(job_id = %id, "Download job paused");
        slot.control.wait_for_idle().await?;
        if was_running {
            let others_active = self.stored_jobs().await?.iter().any(|job| {
                job.id != id
                    && matches!(
                        job.status,
                        DownloadJobStatus::Running | DownloadJobStatus::Queued
                    )
            });
            if others_active {
                self.runtime
                    .database
                    .reorder_download_jobs(&self.runtime.game, &[id.to_string()])
                    .await?;
            } else {
                self.runtime
                    .database
                    .prioritize_download_job(&self.runtime.game, id)
                    .await?;
            }
        }
        self.release_turn(id).await;
        Ok(slot.snapshot.lock().await.clone())
    }
    pub async fn resume(self: &Arc<Self>, id: &str) -> Result<DownloadJobSnapshot> {
        let _starts = self.starts.lock().await;
        self.accept_work()?;
        let was_paused = self.all_paused.load(Ordering::SeqCst);
        self.admission_blocked.store(true, Ordering::SeqCst);
        let result = async {
            let order: Vec<_> = self
                .stored_jobs()
                .await?
                .into_iter()
                .filter(|job| {
                    matches!(
                        job.status,
                        DownloadJobStatus::Queued | DownloadJobStatus::Paused
                    )
                })
                .map(|job| job.id)
                .collect();
            let snapshot = self.resume_locked(id).await?;
            self.restore_waiting_order(&order).await?;
            Ok(snapshot)
        }
        .await;
        self.all_paused
            .store(result.is_err() && was_paused, Ordering::SeqCst);
        self.admission_blocked.store(false, Ordering::SeqCst);
        self.queue_changed.notify_waiters();
        result
    }

    async fn resume_locked(self: &Arc<Self>, id: &str) -> Result<DownloadJobSnapshot> {
        self.accept_work()?;
        if let Some(slot) = self.active_job(id).await {
            slot.control.clear_pause_request();
            if slot.control.state() == DownloadState::Paused {
                slot.control.wait_for_idle().await?;
            }
            self.accept_work()?;
            if slot.control.state() == DownloadState::Cancelled {
                return Err(CoreError::DownloadJobTransition);
            }
            let mut snapshot = slot.snapshot.lock().await;
            if snapshot.status != DownloadJobStatus::Paused {
                if matches!(
                    snapshot.status,
                    DownloadJobStatus::Running | DownloadJobStatus::Queued
                ) {
                    slot.control.resume();
                }
                return Ok(snapshot.clone());
            }
            let instance_job = snapshot.instance_folder.is_some();
            snapshot.status = if instance_job {
                DownloadJobStatus::Queued
            } else {
                DownloadJobStatus::Running
            };
            snapshot.error = None;
            self.persist(&mut snapshot).await?;
            self.emit(snapshot.clone());
            tracing::info!(job_id = %id, "Download job resumed");
            if instance_job {
                self.installations
                    .lock()
                    .await
                    .waiting
                    .push_back(id.to_string());
                slot.enabled.send_replace(true);
                self.queue_changed.notify_waiters();
            } else {
                slot.control.resume();
                slot.enabled.send_replace(true);
            }
            return Ok(snapshot.clone());
        }
        let mut snapshot = self.stored_job(id).await?;
        if matches!(
            snapshot.status,
            DownloadJobStatus::Running | DownloadJobStatus::Queued
        ) {
            snapshot.status = DownloadJobStatus::Interrupted;
            self.persist(&mut snapshot).await?;
        }
        if !matches!(
            snapshot.status,
            DownloadJobStatus::Paused | DownloadJobStatus::Interrupted | DownloadJobStatus::Failed
        ) {
            return Err(CoreError::DownloadJobTransition);
        }
        if let Some(folder) = &snapshot.instance_folder {
            load_instance(
                &self.runtime.database,
                &self.runtime.game.instance_dir(folder)?,
            )
            .await?;
        }
        snapshot.status = DownloadJobStatus::Queued;
        snapshot.error = None;
        self.persist(&mut snapshot).await?;
        self.spawn(snapshot.clone()).await;
        Ok(snapshot)
    }
    pub async fn ensure_launchable(&self, folder: &str) -> Result<()> {
        if self
            .stored_jobs()
            .await?
            .iter()
            .any(|job| job.instance_folder.as_deref() == Some(folder))
        {
            return Err(CoreError::InstancePreparing);
        }
        crate::instance_version_for_launch(
            &load_instance(
                &self.runtime.database,
                &self.runtime.game.instance_dir(folder)?,
            )
            .await?,
        )?;
        Ok(())
    }
    async fn wait(&self, id: &str) -> Result<DownloadJobSnapshot> {
        if let Some(slot) = self.active_job(id).await {
            return Self::wait_slot(&slot).await;
        }
        self.stored_job(id).await
    }
    async fn wait_slot(slot: &ActiveJob) -> Result<DownloadJobSnapshot> {
        let mut receiver = slot.completion.subscribe();
        let outcome = receiver
            .wait_for(Option::is_some)
            .await
            .map_err(|_| CoreError::Io(std::io::Error::other("download worker closed")))?
            .clone()
            .ok_or_else(|| CoreError::Io(std::io::Error::other("download outcome is missing")))?;
        outcome.map_err(|detail| CoreError::DownloadJobExecution { detail })
    }
    async fn active_job(&self, id: &str) -> Option<Arc<ActiveJob>> {
        self.active.lock().await.get(id).cloned()
    }
    pub async fn install_java(self: &Arc<Self>, major: u32) -> Result<Option<String>> {
        let starts = self.starts.lock().await;
        self.accept_work()?;
        let existing = self
            .stored_jobs()
            .await?
            .into_iter()
            .find(|job| job.java_major == Some(major));
        let job = if let Some(job) = existing {
            drop(starts);
            self.resume(&job.id).await?
        } else {
            let job = self
                .runtime
                .database
                .create_java_download_job(&self.runtime.game, major)
                .await?;
            self.spawn(job.clone()).await;
            drop(starts);
            job
        };
        let outcome = self.wait(&job.id).await?;
        match outcome.status {
            DownloadJobStatus::Completed => {
                let paths =
                    crate::runtime_java_paths(&self.runtime.game, &self.runtime.environment)?;
                let path = match major {
                    8 => paths.java8,
                    17 => paths.java17,
                    21 => paths.java21,
                    25 => paths.java25,
                    _ => None,
                };
                Ok(Some(path.ok_or(CoreError::JavaRuntimeMissing)?))
            }
            DownloadJobStatus::Cancelled => Ok(None),
            _ => Err(CoreError::DownloadJobExecution {
                detail: outcome
                    .error
                    .unwrap_or_else(|| "Java installation failed".to_string()),
            }),
        }
    }
    pub async fn suspend_all(&self) -> Result<()> {
        self.closing.store(true, Ordering::SeqCst);
        let _starts = self.starts.lock().await;
        let slots: Vec<_> = self.active.lock().await.values().cloned().collect();
        for slot in &slots {
            slot.control.pause();
        }
        let mut failure = None;
        let mut waiting = Vec::new();
        for slot in slots {
            let mut snapshot = slot.snapshot.lock().await;
            if matches!(
                snapshot.status,
                DownloadJobStatus::Completed
                    | DownloadJobStatus::Cancelled
                    | DownloadJobStatus::Failed
            ) {
                continue;
            }
            slot.suspending.store(true, Ordering::SeqCst);
            if matches!(
                snapshot.status,
                DownloadJobStatus::Running | DownloadJobStatus::Queued
            ) {
                snapshot.status = DownloadJobStatus::Interrupted;
                match self.persist(&mut snapshot).await {
                    Ok(()) => self.emit(snapshot.clone()),
                    Err(error) => {
                        if failure.is_none() {
                            failure = Some(error);
                        }
                    }
                }
            }
            if let Some(worker) = slot.worker.lock().await.as_ref() {
                worker.abort();
            }
            drop(snapshot);
            waiting.push(slot);
        }
        for slot in waiting {
            let mut stopped = slot.stopped.subscribe();
            stopped
                .wait_for(|stopped| *stopped)
                .await
                .map_err(|_| CoreError::Io(std::io::Error::other("shutdown worker closed")))?;
            slot.completion.send_if_modified(|outcome| {
                if outcome.is_some() {
                    return false;
                }
                *outcome = Some(Err("download suspended for shutdown".into()));
                true
            });
            let id = slot.snapshot.lock().await.id.clone();
            self.active.lock().await.remove(&id);
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    fn accept_work(&self) -> Result<()> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(CoreError::DownloadJobExecution {
                detail: "downloads are suspended for shutdown".to_string(),
            });
        }
        Ok(())
    }

    async fn stored_job(&self, id: &str) -> Result<DownloadJobSnapshot> {
        self.runtime
            .database
            .get_download_job(&self.runtime.game, id)
            .await
    }

    async fn stored_jobs(&self) -> Result<Vec<DownloadJobSnapshot>> {
        self.runtime
            .database
            .list_download_jobs(&self.runtime.game)
            .await
    }

    async fn cleanup_artifacts(&self, id: &str) -> Result<()> {
        for path in self
            .runtime
            .database
            .download_cleanup_candidates(&self.runtime.game, id)
            .await?
        {
            super::download::remove_partial_if_idle(&path).await?;
        }
        if let Some(folder) = self.stored_job(id).await?.instance_folder {
            crate::modpacks::workspace::cleanup(&self.runtime.game.instance_dir(&folder)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
