use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::future::BoxFuture;
use tokio::sync::{Mutex, broadcast, mpsc, watch};

use crate::{
    CoreError, DownloadControl, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus,
    DownloadJobUpdate, DownloadPhase, DownloadProgress, DownloadState, InstallationContext,
    InstanceEntry, InstanceSource, LaunchEnvironment, Result, SharedCache,
    create_pending_download_job, create_pending_instance, get_download_job, global_launch,
    list_download_jobs, load_instance, load_settings, update_download_job,
};

use super::installation::InstallationEvent;

#[derive(Clone)]
pub(crate) struct DownloadRuntime {
    pub settings: SharedCache,
    pub game: SharedCache,
    pub environment: LaunchEnvironment,
    pub client: reqwest::Client,
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
    dirty: AtomicBool,
}

pub struct DownloadManager {
    runtime: DownloadRuntime,
    executor: Executor,
    starts: Mutex<()>,
    active: Mutex<HashMap<String, Arc<ActiveJob>>>,
    events: broadcast::Sender<DownloadJobSnapshot>,
    closing: AtomicBool,
}

impl DownloadManager {
    pub fn new(
        settings: SharedCache,
        game: SharedCache,
        environment: LaunchEnvironment,
        client: reqwest::Client,
    ) -> Self {
        let (events, _) = broadcast::channel(512);
        Self {
            runtime: DownloadRuntime {
                settings,
                game,
                environment,
                client,
            },
            executor: super::pipeline::execute,
            starts: Mutex::new(()),
            active: Mutex::new(HashMap::new()),
            events,
            closing: AtomicBool::new(false),
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
        let settings = load_settings(&self.runtime.settings)?;
        let root = self.runtime.game.instances_dir()?;
        let entry = create_pending_instance(&root, name, source, &global_launch(&settings))?;
        self.start(&entry.folder, DownloadJobOperation::Install)
            .await?;
        Ok(entry)
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
        if let Some(job) = list_download_jobs(&self.runtime.settings, &self.runtime.game)?
            .into_iter()
            .find(|job| job.instance_folder.as_deref() == Some(folder))
        {
            drop(starts);
            return match job.status {
                DownloadJobStatus::Paused
                | DownloadJobStatus::Interrupted
                | DownloadJobStatus::Failed => self.resume(&job.id).await,
                _ => Ok(job),
            };
        }
        let profile = load_instance(&self.runtime.game.instance_dir(folder)?)?;
        let job = create_pending_download_job(
            &self.runtime.settings,
            &self.runtime.game,
            folder,
            profile.version_id.as_deref(),
            operation,
        )?;
        self.spawn(job.clone()).await;
        Ok(job)
    }

    async fn spawn(self: &Arc<Self>, snapshot: DownloadJobSnapshot) {
        let (completion, _) = watch::channel(None);
        let (stopped, _) = watch::channel(false);
        let slot = Arc::new(ActiveJob {
            control: DownloadControl::new(),
            snapshot: Mutex::new(snapshot.clone()),
            completion,
            stopped,
            worker: Mutex::new(None),
            suspending: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
        });
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
            let failure = match worker.await {
                Ok(Ok(())) => {
                    slot.stopped.send_replace(true);
                    return;
                }
                Ok(Err(error)) => error.to_string(),
                Err(_) => "download worker stopped unexpectedly".to_string(),
            };
            if slot.suspending.load(Ordering::SeqCst) {
                let id = slot.snapshot.lock().await.id.clone();
                slot.completion
                    .send_replace(Some(Err("download suspended for shutdown".to_string())));
                manager.active.lock().await.remove(&id);
                slot.stopped.send_replace(true);
                return;
            }
            manager.fail(Arc::clone(&slot), failure).await;
            slot.stopped.send_replace(true);
        });
    }

    async fn drive(self: &Arc<Self>, id: String, slot: Arc<ActiveJob>) -> Result<()> {
        let initial = slot.snapshot.lock().await.clone();
        let (events, mut incoming) = mpsc::channel(4096);
        let context = InstallationContext::for_job(
            self.runtime.settings.clone(),
            self.runtime.game.clone(),
            id.clone(),
            slot.control.clone(),
            events,
        );
        let operation = (self.executor)(self.runtime.clone(), initial, context);
        tokio::pin!(operation);
        let mut last_emit = Instant::now();
        let mut last_save = Instant::now();
        let mut observing = true;
        let mut ticks = tokio::time::interval(Duration::from_millis(100));
        let result = loop {
            tokio::select! {
                event = incoming.recv(), if observing => {
                    if let Some(event) = event {
                        self.apply_event(&slot, event, &mut last_emit, &mut last_save).await?;
                    } else { observing = false; }
                }
                result = &mut operation => break result,
                _ = ticks.tick() => {
                    self.flush_progress(&slot, &mut last_emit, &mut last_save).await?;
                }
            }
        };
        while let Ok(event) = incoming.try_recv() {
            self.apply_event(&slot, event, &mut last_emit, &mut last_save)
                .await?;
        }
        let mut snapshot = slot.snapshot.lock().await;
        let result = if slot.control.state() == DownloadState::Cancelled {
            Err(CoreError::DownloadCancelled)
        } else {
            result
        };
        if matches!(result, Err(CoreError::DownloadCancelled)) {
            super::jobs::cleanup_artifacts(&self.runtime.settings, &self.runtime.game, &id).await?;
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
        self.persist(&mut snapshot)?;
        self.emit(snapshot.clone());
        if snapshot.status == DownloadJobStatus::Cancelling {
            // The cancel command still needs to remove the instance directory.
            return Ok(());
        }
        slot.completion.send_replace(Some(Ok(snapshot.clone())));
        drop(snapshot);
        self.active.lock().await.remove(&id);
        Ok(())
    }

    async fn apply_event(
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
                super::jobs::set_job_version(
                    &self.runtime.settings,
                    &self.runtime.game,
                    &snapshot.id,
                    &version,
                )?;
                snapshot.version_id = Some(version);
                true
            }
            InstallationEvent::Progress(progress) => {
                snapshot.progress = progress;
                false
            }
        };
        snapshot.status = if slot.control.state() == DownloadState::Paused {
            DownloadJobStatus::Paused
        } else {
            DownloadJobStatus::Running
        };
        slot.dirty.store(true, Ordering::SeqCst);
        if structural || last_save.elapsed() >= Duration::from_secs(1) {
            self.persist(&mut snapshot)?;
            *last_save = Instant::now();
        }
        if structural || last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit(snapshot.clone());
            slot.dirty.store(false, Ordering::SeqCst);
            *last_emit = Instant::now();
        }
        Ok(())
    }

    pub async fn pause(&self, id: &str) -> Result<DownloadJobSnapshot> {
        let Some(slot) = self.active_job(id).await else {
            let snapshot = get_download_job(&self.runtime.settings, &self.runtime.game, id)?;
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
        if snapshot.status != DownloadJobStatus::Queued
            && (snapshot.status != DownloadJobStatus::Running
                || snapshot.phase != DownloadPhase::Downloading)
        {
            return Err(CoreError::DownloadJobTransition);
        }
        slot.control.pause();
        snapshot.status = DownloadJobStatus::Paused;
        self.persist(&mut snapshot)?;
        self.emit(snapshot.clone());
        drop(snapshot);
        slot.control.wait_for_idle().await?;
        Ok(slot.snapshot.lock().await.clone())
    }

    pub async fn resume(self: &Arc<Self>, id: &str) -> Result<DownloadJobSnapshot> {
        let _starts = self.starts.lock().await;
        self.accept_work()?;
        if let Some(slot) = self.active_job(id).await {
            if slot.control.state() == DownloadState::Paused {
                slot.control.wait_for_idle().await?;
            }
            self.accept_work()?;
            if slot.control.state() == DownloadState::Cancelled {
                return Err(CoreError::DownloadJobTransition);
            }
            let mut snapshot = slot.snapshot.lock().await;
            if snapshot.status != DownloadJobStatus::Paused {
                return Ok(snapshot.clone());
            }
            slot.control.resume();
            snapshot.status = DownloadJobStatus::Running;
            snapshot.error = None;
            self.persist(&mut snapshot)?;
            self.emit(snapshot.clone());
            return Ok(snapshot.clone());
        }
        let mut snapshot = get_download_job(&self.runtime.settings, &self.runtime.game, id)?;
        if matches!(
            snapshot.status,
            DownloadJobStatus::Running | DownloadJobStatus::Queued
        ) {
            snapshot.status = DownloadJobStatus::Interrupted;
            self.persist(&mut snapshot)?;
        }
        if !matches!(
            snapshot.status,
            DownloadJobStatus::Paused | DownloadJobStatus::Interrupted | DownloadJobStatus::Failed
        ) {
            return Err(CoreError::DownloadJobTransition);
        }
        if let Some(folder) = &snapshot.instance_folder {
            load_instance(&self.runtime.game.instance_dir(folder)?)?;
        }
        snapshot.status = DownloadJobStatus::Queued;
        snapshot.error = None;
        self.persist(&mut snapshot)?;
        self.spawn(snapshot.clone()).await;
        Ok(snapshot)
    }

    pub async fn cancel(&self, id: &str) -> Result<DownloadJobSnapshot> {
        let _starts = self.starts.lock().await;
        let job = if let Some(slot) = self.active_job(id).await {
            let mut snapshot = slot.snapshot.lock().await;
            let next = self.prepare_cancellation(snapshot.clone())?;
            *snapshot = next;
            if snapshot.status == DownloadJobStatus::Cancelling {
                slot.control.cancel();
            }
            snapshot.clone()
        } else {
            self.prepare_cancellation(get_download_job(
                &self.runtime.settings,
                &self.runtime.game,
                id,
            )?)?
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
            Err(error) => match self.record_cancellation_failure(id, &error) {
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

    fn prepare_cancellation(&self, mut job: DownloadJobSnapshot) -> Result<DownloadJobSnapshot> {
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
                self.persist(&mut job)?;
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
        job.status = DownloadJobStatus::Cancelled;
        job.error = None;
        self.persist(&mut job)?;
        self.emit(job.clone());
        Ok(job)
    }

    fn record_cancellation_failure(&self, id: &str, error: &CoreError) -> Result<()> {
        let mut job = get_download_job(&self.runtime.settings, &self.runtime.game, id)?;
        if job.status == DownloadJobStatus::Cancelling {
            job.error = Some(error.to_string());
            self.persist(&mut job)?;
            self.emit(job);
        }
        Ok(())
    }

    async fn cancel_job(&self, id: &str) -> Result<DownloadJobSnapshot> {
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
        let mut snapshot = get_download_job(&self.runtime.settings, &self.runtime.game, id)?;
        if matches!(
            snapshot.status,
            DownloadJobStatus::Completed | DownloadJobStatus::Cancelled
        ) {
            return Ok(snapshot);
        }
        super::jobs::cleanup_artifacts(&self.runtime.settings, &self.runtime.game, id).await?;
        if snapshot.status != DownloadJobStatus::Cancelling {
            snapshot.status = DownloadJobStatus::Cancelled;
        }
        snapshot.error = None;
        self.persist(&mut snapshot)?;
        self.emit(snapshot.clone());
        Ok(snapshot)
    }

    pub async fn cancel_instance(&self, folder: &str) -> Result<()> {
        let _starts = self.starts.lock().await;
        let jobs = list_download_jobs(&self.runtime.settings, &self.runtime.game)?;
        for job in jobs
            .into_iter()
            .filter(|job| job.instance_folder.as_deref() == Some(folder))
        {
            let mut stopped = self.cancel_job(&job.id).await?;
            if stopped.status == DownloadJobStatus::Cancelling {
                // The explicit instance-delete command owns directory removal.
                stopped.status = DownloadJobStatus::Cancelled;
                self.persist(&mut stopped)?;
                self.emit(stopped.clone());
                self.complete_cancellation(&job.id, &Ok(stopped)).await;
            }
        }
        Ok(())
    }

    pub fn ensure_launchable(&self, folder: &str) -> Result<()> {
        if list_download_jobs(&self.runtime.settings, &self.runtime.game)?
            .iter()
            .any(|job| job.instance_folder.as_deref() == Some(folder))
        {
            return Err(CoreError::InstancePreparing);
        }
        crate::instance_version_for_launch(&load_instance(
            &self.runtime.game.instance_dir(folder)?,
        )?)?;
        Ok(())
    }

    pub fn change_version(&self, folder: &str, version: &str) -> Result<InstanceEntry> {
        self.ensure_launchable(folder)?;
        crate::update_instance_version(&self.runtime.game.instances_dir()?, folder, version)
    }

    async fn wait(&self, id: &str) -> Result<DownloadJobSnapshot> {
        if let Some(slot) = self.active_job(id).await {
            return Self::wait_slot(&slot).await;
        }
        get_download_job(&self.runtime.settings, &self.runtime.game, id)
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

    async fn fail(&self, slot: Arc<ActiveJob>, detail: String) {
        slot.control.cancel();
        let mut snapshot = slot.snapshot.lock().await;
        if snapshot.status != DownloadJobStatus::Cancelling {
            snapshot.status = DownloadJobStatus::Failed;
        }
        snapshot.error = Some(detail.clone());
        let outcome = match self.persist(&mut snapshot) {
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

    fn persist(&self, snapshot: &mut DownloadJobSnapshot) -> Result<()> {
        *snapshot = update_download_job(
            &self.runtime.settings,
            &self.runtime.game,
            &snapshot.id,
            DownloadJobUpdate {
                status: snapshot.status,
                component: snapshot.component,
                phase: snapshot.phase,
                progress: snapshot.progress,
                error: snapshot.error.clone(),
            },
        )?;
        Ok(())
    }

    fn emit(&self, snapshot: DownloadJobSnapshot) {
        let _optional_observers = self.events.send(snapshot);
    }

    async fn active_job(&self, id: &str) -> Option<Arc<ActiveJob>> {
        self.active.lock().await.get(id).cloned()
    }

    pub async fn install_java(self: &Arc<Self>, major: u32) -> Result<Option<String>> {
        let starts = self.starts.lock().await;
        self.accept_work()?;
        let existing = list_download_jobs(&self.runtime.settings, &self.runtime.game)?
            .into_iter()
            .find(|job| job.java_major == Some(major));
        let job = if let Some(job) = existing {
            drop(starts);
            self.resume(&job.id).await?
        } else {
            let job =
                super::jobs::create_java_job(&self.runtime.settings, &self.runtime.game, major)?;
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
                match self.persist(&mut snapshot) {
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

    async fn flush_progress(
        &self,
        slot: &ActiveJob,
        last_emit: &mut Instant,
        last_save: &mut Instant,
    ) -> Result<()> {
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
            self.persist(&mut snapshot)?;
            *last_save = Instant::now();
        }
        self.emit(snapshot.clone());
        slot.dirty.store(false, Ordering::SeqCst);
        *last_emit = Instant::now();
        Ok(())
    }

    fn accept_work(&self) -> Result<()> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(CoreError::DownloadJobExecution {
                detail: "downloads are suspended for shutdown".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

fn empty_progress() -> DownloadProgress {
    DownloadProgress {
        finished: 0,
        failed: 0,
        total: 0,
        bytes_done: 0,
        bytes_total: 0,
    }
}
