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
    transfers: watch::Sender<usize>,
}

pub(crate) struct TransferActivity {
    transfers: watch::Sender<usize>,
}

impl Drop for TransferActivity {
    fn drop(&mut self) {
        self.transfers
            .send_modify(|count| *count = count.saturating_sub(1));
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadControl {
    pub fn new() -> Self {
        let (state, _) = watch::channel(DownloadState::Running);
        let (transfers, _) = watch::channel(0);
        Self { state, transfers }
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

    pub(crate) async fn cancelled(&self) {
        let mut receiver = self.state.subscribe();
        let _closed_or_cancelled = receiver
            .wait_for(|state| *state == DownloadState::Cancelled)
            .await;
    }

    pub(crate) fn begin_transfer(&self) -> Result<TransferActivity> {
        // Hold the state read lock while admitting a transfer, so pause cannot
        // acknowledge an idle queue just before a new request is admitted.
        let state = self.state.borrow();
        match *state {
            DownloadState::Paused => return Err(CoreError::DownloadPaused),
            DownloadState::Cancelled => return Err(CoreError::DownloadCancelled),
            DownloadState::Running => {}
        }
        self.transfers.send_modify(|count| *count += 1);
        Ok(TransferActivity {
            transfers: self.transfers.clone(),
        })
    }

    pub(crate) async fn wait_for_idle(&self) -> Result<()> {
        let mut transfers = self.transfers.subscribe();
        transfers
            .wait_for(|count| *count == 0)
            .await
            .map_err(|_| CoreError::DownloadCancelled)?;
        Ok(())
    }
}
