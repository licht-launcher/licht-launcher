use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StartupPhase {
    #[default]
    Preparing,
    CheckingUpdates,
    DownloadingUpdate,
    VerifyingUpdate,
    InstallingUpdate,
    MigratingDatabase,
    MigratingData,
    MigratingInstances,
    MigratingSettings,
    RestoringCache,
    StartingServices,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StartupUpdateStatus {
    #[default]
    NotChecked,
    UpToDate,
    Available,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartupProgress {
    pub phase: StartupPhase,
    #[ts(type = "number")]
    pub processed_files: u64,
    #[ts(type = "number")]
    pub processed_bytes: u64,
    #[ts(type = "number | null")]
    pub total_bytes: Option<u64>,
    pub version: Option<String>,
    pub update_status: StartupUpdateStatus,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub preview: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StartupPreviewScenario {
    NoUpdate,
    Update,
    Migration,
    UpdateAndMigration,
    CheckFailure,
    MigrationFailure,
}

struct ReporterState {
    snapshot: StartupProgress,
    last: Instant,
}

#[derive(Clone)]
pub struct StartupReporter {
    state: Arc<Mutex<ReporterState>>,
    publish: Arc<dyn Fn(StartupProgress) -> Result<()> + Send + Sync>,
}

impl Default for StartupReporter {
    fn default() -> Self {
        Self::new(|_| Ok(()))
    }
}

impl StartupReporter {
    pub fn new(publish: impl Fn(StartupProgress) -> Result<()> + Send + Sync + 'static) -> Self {
        Self {
            state: Arc::new(Mutex::new(ReporterState {
                snapshot: StartupProgress::default(),
                last: Instant::now(),
            })),
            publish: Arc::new(publish),
        }
    }

    pub fn phase(&self, phase: StartupPhase) -> Result<()> {
        self.change(Some(phase), None, true)
    }

    pub(crate) fn file(&self, size: u64) -> Result<()> {
        self.change(None, Some(size), false)
    }

    pub(crate) fn checkpoint(&self) -> Result<()> {
        self.change(None, None, false)
    }

    fn change(&self, phase: Option<StartupPhase>, size: Option<u64>, force: bool) -> Result<()> {
        let snapshot = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| std::io::Error::other("startup progress lock failed"))?;
            if let Some(phase) = phase {
                state.snapshot.phase = phase;
            }
            if let Some(size) = size {
                state.snapshot.processed_files += 1;
                state.snapshot.processed_bytes += size;
            }
            if !force && state.last.elapsed() < Duration::from_millis(100) {
                return Ok(());
            }
            state.last = Instant::now();
            state.snapshot.clone()
        };
        (self.publish)(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_retains_counts_between_phases_and_propagates_stop_requests() {
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let output = Arc::clone(&snapshots);
        let reporter = StartupReporter::new(move |snapshot| {
            output.lock().expect("snapshots").push(snapshot);
            Ok(())
        });
        reporter.phase(StartupPhase::MigratingData).expect("phase");
        reporter.file(1024).expect("file");
        reporter
            .phase(StartupPhase::MigratingInstances)
            .expect("phase");
        let output = snapshots.lock().expect("snapshots");
        assert_eq!(output.len(), 2);
        assert_eq!(output[1].processed_files, 1);
        assert_eq!(output[1].processed_bytes, 1024);
        assert_eq!(output[1].total_bytes, None);
        let stopped = StartupReporter::new(|_| Err(std::io::Error::other("stopped").into()));
        assert!(stopped.phase(StartupPhase::Preparing).is_err());
    }
}
