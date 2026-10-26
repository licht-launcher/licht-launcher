use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::watch;

use crate::{CoreError, Result};

#[derive(Debug, Default)]
pub(crate) struct TransferCounters {
    received: AtomicU64,
    written: AtomicU64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTelemetry {
    #[ts(type = "number")]
    pub received_bytes: u64,
    #[ts(type = "number")]
    pub written_bytes: u64,
    pub paused: bool,
}

impl TransferCounters {
    pub(crate) fn snapshot(&self, paused: bool) -> DownloadTelemetry {
        DownloadTelemetry {
            received_bytes: self.received.load(Ordering::Relaxed),
            written_bytes: self.written.load(Ordering::Relaxed),
            paused,
        }
    }
}

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
    counters: Arc<TransferCounters>,
    pause_requested: Arc<AtomicBool>,
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
        Self::with_counters(Arc::new(TransferCounters::default()))
    }

    pub(crate) fn with_counters(counters: Arc<TransferCounters>) -> Self {
        let (state, _) = watch::channel(DownloadState::Running);
        let (transfers, _) = watch::channel(0);
        Self {
            state,
            transfers,
            counters,
            pause_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn received(&self, bytes: u64) {
        self.counters.received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(crate) fn written(&self, bytes: u64) {
        self.counters.written.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(crate) fn request_pause(&self) {
        self.pause_requested.store(true, Ordering::SeqCst);
    }

    pub(crate) fn clear_pause_request(&self) {
        self.pause_requested.store(false, Ordering::SeqCst);
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
        self.pause_requested.store(false, Ordering::SeqCst);
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
        if self.pause_requested.load(Ordering::SeqCst) {
            self.pause();
        }
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
