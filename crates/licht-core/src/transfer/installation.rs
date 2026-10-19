use std::future::Future;

use tokio::sync::mpsc;

use super::download::{CancellationCleanup, download_verified_all_with_cleanup};
use crate::{
    CoreError, DownloadComponent, DownloadControl, DownloadOptions, DownloadPhase,
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
    job: Option<(SharedCache, SharedCache, String)>,
}

impl Default for InstallationContext {
    fn default() -> Self {
        Self {
            control: DownloadControl::new(),
            component: DownloadComponent::Minecraft,
            legacy: None,
            events: None,
            job: None,
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
    pub(crate) fn options(&self) -> Result<DownloadOptions> {
        match &self.job {
            Some((settings, game, _)) => Ok(DownloadOptions {
                limits: crate::shared_transfer_limits(),
                download_concurrency: crate::download_concurrency(game, settings)?,
                write_concurrency: crate::write_concurrency(game, settings)?,
                retry: crate::DEFAULT_RETRY,
            }),
            None => Ok(DownloadOptions::default()),
        }
    }
    pub(crate) fn for_job(
        settings: SharedCache,
        game: SharedCache,
        id: String,
        control: DownloadControl,
        events: mpsc::Sender<InstallationEvent>,
    ) -> Self {
        Self {
            control,
            events: Some(events),
            job: Some((settings, game, id)),
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
        if let Some((settings, game, id)) = &self.job {
            let paths: Vec<_> = tasks.iter().map(|task| task.destination.clone()).collect();
            super::jobs::register_artifacts(settings, game, id, &paths)?;
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
        );
        tokio::pin!(download);
        let mut observing = true;
        loop {
            tokio::select! {
                progress = incoming.recv(), if observing => match progress {
                    Some(progress) => self.send(progress).await?,
                    None => observing = false,
                },
                result = &mut download => {
                    while let Ok(progress) = incoming.try_recv() { self.send(progress).await?; }
                    return result;
                }
            }
        }
    }
}
