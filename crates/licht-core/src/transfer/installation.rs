use std::future::Future;

use tokio::sync::mpsc;

use super::download::{CancellationCleanup, SizePolicy, download_verified_all_with_cleanup};
use crate::{
    CoreError, DatabaseClient, DownloadComponent, DownloadControl, DownloadOptions, DownloadPhase,
    DownloadProgress, DownloadTask, Result, SharedCache, VerifiedDownloadTask,
};

#[derive(Debug)]
pub(crate) enum InstallationEvent {
    Stage(DownloadComponent, DownloadPhase),
    Progress(DownloadProgress),
    Version(String),
}

#[derive(Clone)]
pub struct InstallationContext {
    pub(crate) control: DownloadControl,
    component: DownloadComponent,
    legacy: Option<mpsc::Sender<DownloadProgress>>,
    events: Option<mpsc::Sender<InstallationEvent>>,
    job: Option<InstallationJob>,
    size_policy: SizePolicy,
}

#[derive(Clone)]
struct InstallationJob {
    settings: crate::Database,
    game: SharedCache,
    id: String,
    database: DatabaseClient,
}

impl Default for InstallationContext {
    fn default() -> Self {
        Self {
            control: DownloadControl::new(),
            component: DownloadComponent::Minecraft,
            legacy: None,
            events: None,
            job: None,
            size_policy: SizePolicy::Exact,
        }
    }
}

impl From<mpsc::Sender<DownloadProgress>> for InstallationContext {
    fn from(progress: mpsc::Sender<DownloadProgress>) -> Self {
        Self {
            legacy: Some(progress),
            ..Self::default()
        }
    }
}

impl InstallationContext {
    pub(crate) fn with_size_hints(&self) -> Self {
        Self {
            size_policy: SizePolicy::Hint,
            ..self.clone()
        }
    }

    pub(crate) async fn options(&self) -> Result<DownloadOptions> {
        match &self.job {
            Some(job) => crate::settings::download_options(&job.game, &job.settings).await,
            None => Ok(DownloadOptions::default()),
        }
    }
    pub(crate) fn for_job(
        settings: crate::Database,
        game: SharedCache,
        id: String,
        control: DownloadControl,
        events: mpsc::Sender<InstallationEvent>,
        database: DatabaseClient,
    ) -> Self {
        Self {
            control,
            events: Some(events),
            job: Some(InstallationJob {
                settings,
                game,
                id,
                database,
            }),
            ..Self::default()
        }
    }

    pub(crate) fn component(&self, component: DownloadComponent) -> Self {
        Self {
            component,
            ..self.clone()
        }
    }

    pub(crate) async fn checkpoint(&self) -> Result<()> {
        self.control.ready().await
    }

    pub(crate) async fn stage(&self, phase: DownloadPhase) -> Result<()> {
        self.checkpoint().await?;
        self.event(InstallationEvent::Stage(self.component, phase))
            .await?;
        tokio::task::yield_now().await;
        self.checkpoint().await
    }

    pub(crate) async fn resolved_version(&self, version: &str) -> Result<()> {
        self.event(InstallationEvent::Version(version.to_string()))
            .await
    }

    pub(crate) async fn metadata<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
        self.checkpoint().await?;
        tokio::select! {
            biased;
            _ = self.control.cancelled() => Err(CoreError::DownloadCancelled),
            result = future => result,
        }
    }

    async fn event(&self, event: InstallationEvent) -> Result<()> {
        if let Some(events) = &self.events {
            events
                .send(event)
                .await
                .map_err(|_| CoreError::DownloadCancelled)?;
        }
        Ok(())
    }

    pub(crate) async fn send(&self, progress: DownloadProgress) -> Result<()> {
        // Legacy progress is observational: a closed receiver must not cancel an install.
        if let Some(legacy) = &self.legacy {
            let _optional_observer = legacy.try_send(progress);
        }
        self.event(InstallationEvent::Progress(progress)).await
    }

    pub(crate) fn try_send(&self, progress: DownloadProgress) -> Result<()> {
        if let Some(legacy) = &self.legacy {
            let _optional_observer = legacy.try_send(progress);
        }
        if let Some(events) = &self.events {
            match events.try_send(InstallationEvent::Progress(progress)) {
                Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(CoreError::DownloadCancelled);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn queue(
        &self,
        client: &reqwest::Client,
        tasks: &[DownloadTask],
        options: DownloadOptions,
    ) -> Result<()> {
        let tasks: Vec<_> = tasks
            .iter()
            .map(|task| VerifiedDownloadTask {
                url: task.url.clone(),
                destination: task.destination.clone(),
                checksum: crate::DownloadChecksum::legacy_sha1(&task.sha1),
                size: task.size,
            })
            .collect();
        self.verified_queue(client, &tasks, options).await
    }

    pub(crate) async fn verified_queue(
        &self,
        client: &reqwest::Client,
        tasks: &[VerifiedDownloadTask],
        options: DownloadOptions,
    ) -> Result<()> {
        if tasks.is_empty() {
            return Ok(());
        }
        self.stage(DownloadPhase::Downloading).await?;
        if let Some(job) = &self.job {
            let paths: Vec<_> = tasks.iter().map(|task| task.destination.clone()).collect();
            job.database
                .register_download_artifacts(&job.game, &job.id, &paths)
                .await?;
        }
        let (progress, mut incoming) = mpsc::channel(4096);
        let cleanup = if self.job.is_some() {
            CancellationCleanup::DeferToJob
        } else {
            CancellationCleanup::RemovePartial
        };
        let download = download_verified_all_with_cleanup(
            client,
            tasks,
            &options,
            &self.control,
            progress,
            cleanup,
            self.size_policy,
        );
        self.forward_progress(download, &mut incoming).await
    }

    /// Forwards download progress while `download` runs, then drains what is left.
    /// The download is polled first so progress it reports in a poll is forwarded in
    /// that same poll: a paused job stops polling once its transfers go idle.
    async fn forward_progress<T>(
        &self,
        download: impl Future<Output = Result<T>>,
        incoming: &mut mpsc::Receiver<DownloadProgress>,
    ) -> Result<T> {
        tokio::pin!(download);
        let mut observing = true;
        loop {
            tokio::select! {
                biased;
                result = &mut download => {
                    while let Ok(progress) = incoming.try_recv() { self.send(progress).await?; }
                    return result;
                }
                progress = incoming.recv(), if observing => match progress {
                    Some(progress) => self.send(progress).await?,
                    None => observing = false,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt;
    use tokio::sync::mpsc;

    use super::{InstallationContext, InstallationEvent};
    use crate::{DownloadProgress, Result};

    fn progress(bytes_done: u64) -> DownloadProgress {
        DownloadProgress {
            finished: 0,
            failed: 0,
            total: 1,
            bytes_done,
            bytes_total: 11,
        }
    }

    #[test]
    fn progress_reported_while_polling_the_download_is_forwarded_in_that_poll() {
        // A paused job stops polling right after its transfers go idle, so progress
        // left behind in this poll would not reach the job until resume.
        // tokio::select! picks its branch order at random; repeat to cover both orders.
        for _ in 0..64 {
            let (events, mut job_events) = mpsc::channel(4);
            let context = InstallationContext {
                events: Some(events),
                ..InstallationContext::default()
            };
            let (sender, mut incoming) = mpsc::channel(4);
            let download = async move {
                sender.try_send(progress(5)).expect("the channel has room");
                std::future::pending::<Result<()>>().await
            };

            let outcome = context
                .forward_progress(download, &mut incoming)
                .now_or_never();

            assert!(outcome.is_none(), "the download is still running");
            match job_events.try_recv() {
                Ok(InstallationEvent::Progress(forwarded)) => assert_eq!(forwarded.bytes_done, 5),
                other => panic!("progress was not forwarded in the same poll: {other:?}"),
            }
        }
    }
}
