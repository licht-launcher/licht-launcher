use super::invalid;
use crate::{DownloadProgress, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadJobOperation {
    Install,
    Repair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadJobStatus {
    Queued,
    Running,
    Paused,
    Interrupted,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
}
impl DownloadJobStatus {
    pub(super) fn can_transition_to(self, next: Self) -> bool {
        use DownloadJobStatus::*;
        match self {
            Completed | Cancelled => false,
            _ if self == next => true,
            Queued => matches!(
                next,
                Running | Paused | Interrupted | Cancelling | Cancelled | Failed
            ),
            Running => matches!(
                next,
                Paused | Interrupted | Cancelling | Completed | Cancelled | Failed
            ),
            Paused | Interrupted | Failed => {
                matches!(next, Queued | Running | Cancelling | Cancelled)
            }
            Cancelling => matches!(next, Cancelled),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadComponent {
    Minecraft,
    Java,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    Modpack,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadPhase {
    Preparing,
    Downloading,
    Verifying,
    Extracting,
    Installing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobSnapshot {
    pub id: String,
    pub instance_folder: Option<String>,
    pub version_id: Option<String>,
    pub operation: DownloadJobOperation,
    pub status: DownloadJobStatus,
    pub component: Option<DownloadComponent>,
    pub phase: DownloadPhase,
    pub progress: DownloadProgress,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub java_major: Option<u32>,
}

/// Only core callers update lifecycle state; frontend commands cannot set progress.
#[derive(Debug, Clone)]
pub struct DownloadJobUpdate {
    pub status: DownloadJobStatus,
    pub component: Option<DownloadComponent>,
    pub phase: DownloadPhase,
    pub progress: DownloadProgress,
    pub error: Option<String>,
}

pub(super) fn validate_progress(progress: DownloadProgress) -> Result<()> {
    if progress
        .finished
        .checked_add(progress.failed)
        .is_none_or(|count| count > progress.total)
        || (progress.bytes_total > 0 && progress.bytes_done > progress.bytes_total)
    {
        return Err(invalid("progress exceeds its totals"));
    }
    Ok(())
}
pub(super) fn validate_phase(
    phase: DownloadPhase,
    component: Option<DownloadComponent>,
) -> Result<()> {
    if phase != DownloadPhase::Preparing && component.is_none() {
        return Err(invalid("an active phase requires a component"));
    }
    Ok(())
}
pub(super) fn validate_cancellation_target(
    job: &DownloadJobSnapshot,
    status: DownloadJobStatus,
) -> Result<()> {
    if status == DownloadJobStatus::Cancelling
        && (job.operation != DownloadJobOperation::Install || job.instance_folder.is_none())
    {
        return Err(invalid(
            "pending instance removal requires an instance installation",
        ));
    }
    Ok(())
}
pub(super) fn validate_completion(
    status: DownloadJobStatus,
    progress: DownloadProgress,
) -> Result<()> {
    if status == DownloadJobStatus::Completed
        && (progress.failed != 0 || progress.finished != progress.total)
    {
        return Err(invalid(
            "a completed job must have no unfinished or failed files",
        ));
    }
    Ok(())
}
