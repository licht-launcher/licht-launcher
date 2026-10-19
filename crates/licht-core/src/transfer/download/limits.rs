use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use tokio::sync::Notify;

static SHARED_TRANSFER_LIMITS: LazyLock<Arc<TransferLimits>> =
    LazyLock::new(|| Arc::new(TransferLimits::new()));

/// Shared download/write gates for the whole process (all installs).
pub fn shared_transfer_limits() -> Arc<TransferLimits> {
    Arc::clone(&SHARED_TRANSFER_LIMITS)
}

/// Caps how many network fetches or disk writes run at once. Limit is passed
/// on each acquire so Settings changes apply without recreating the gate.
#[derive(Debug, Default)]
pub struct ConcurrencyGate {
    active: StdMutex<usize>,
    waiters: Notify,
}

pub struct GatePermit<'a> {
    gate: &'a ConcurrencyGate,
}

impl Drop for GatePermit<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.gate.active.lock() {
            *active = active.saturating_sub(1);
        }
        self.gate.waiters.notify_waiters();
    }
}

impl ConcurrencyGate {
    pub fn new() -> Self {
        Self {
            active: StdMutex::new(0),
            waiters: Notify::new(),
        }
    }

    pub async fn acquire(&self, limit: usize) -> GatePermit<'_> {
        let limit = limit.max(1);

        loop {
            let notified = self.waiters.notified();
            {
                let mut active = self
                    .active
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if *active < limit {
                    *active += 1;
                    return GatePermit { gate: self };
                }
            }

            notified.await;
        }
    }
}

/// Pair of gates shared across installs (or isolated in tests).
#[derive(Debug, Default)]
pub struct TransferLimits {
    pub download: ConcurrencyGate,
    pub write: ConcurrencyGate,
}

impl TransferLimits {
    pub fn new() -> Self {
        Self {
            download: ConcurrencyGate::new(),
            write: ConcurrencyGate::new(),
        }
    }
}
