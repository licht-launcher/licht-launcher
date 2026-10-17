use tokio::sync::watch;

use crate::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Running,
    Paused,
    Cancelled,
}

/// Shared control for one transfer or queue. Cancellation is terminal.
#[derive(Debug, Clone)]
pub struct DownloadControl {
    state: watch::Sender<DownloadState>,
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadControl {
    pub fn new() -> Self {
        let (state, _) = watch::channel(DownloadState::Running);
        Self { state }
    }

    pub fn state(&self) -> DownloadState {
        *self.state.borrow()
    }

    pub fn pause(&self) {
        self.state.send_if_modified(|state| {
            if *state != DownloadState::Running {
                return false;
            }
            *state = DownloadState::Paused;
            true
        });
    }

    pub fn resume(&self) {
        self.state.send_if_modified(|state| {
            if *state != DownloadState::Paused {
                return false;
            }
            *state = DownloadState::Running;
            true
        });
    }

    pub fn cancel(&self) {
        self.state.send_replace(DownloadState::Cancelled);
    }

    pub(crate) async fn ready(&self) -> Result<()> {
        let mut receiver = self.state.subscribe();
        let state = *receiver
            .wait_for(|state| *state != DownloadState::Paused)
            .await
            .map_err(|_| CoreError::DownloadCancelled)?;
        match state {
            DownloadState::Cancelled => Err(CoreError::DownloadCancelled),
            _ => Ok(()),
        }
    }

    pub(crate) async fn interrupted(&self) -> CoreError {
        let mut receiver = self.state.subscribe();
        match receiver
            .wait_for(|state| *state != DownloadState::Running)
            .await
        {
            Ok(state) if *state == DownloadState::Paused => CoreError::DownloadPaused,
            _ => CoreError::DownloadCancelled,
        }
    }
}
