use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{TransferLimits, shared_transfer_limits};
use crate::DownloadChecksum;

pub const DEFAULT_CONCURRENCY: usize = 8;
pub const DEFAULT_RETRY: Retry = Retry {
    attempts: 3,
    initial_backoff: Duration::from_millis(200),
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SizePolicy {
    #[default]
    Exact,
    Hint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retry {
    pub attempts: u32,
    pub initial_backoff: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub destination: PathBuf,
    pub sha1: String,
    /// Size published by Mojang. The progress total is the sum of these sizes.
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub finished: u32,
    pub failed: u32,
    pub total: u32,
    // JSON numbers. A download size fits in a JavaScript number, unlike `bigint`.
    #[ts(type = "number")]
    pub bytes_done: u64,
    #[ts(type = "number")]
    pub bytes_total: u64,
}

/// Download progress tagged with the version (or Java major) being installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub version_id: String,
    pub finished: u32,
    pub failed: u32,
    pub total: u32,
    #[ts(type = "number")]
    pub bytes_done: u64,
    #[ts(type = "number")]
    pub bytes_total: u64,
}

impl InstallProgress {
    pub fn tagged(version_id: impl Into<String>, progress: DownloadProgress) -> Self {
        Self {
            version_id: version_id.into(),
            finished: progress.finished,
            failed: progress.failed,
            total: progress.total,
            bytes_done: progress.bytes_done,
            bytes_total: progress.bytes_total,
        }
    }

    pub fn download(&self) -> DownloadProgress {
        DownloadProgress {
            finished: self.finished,
            failed: self.failed,
            total: self.total,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub limits: Arc<TransferLimits>,
    pub download_concurrency: usize,
    pub write_concurrency: usize,
    pub retry: Retry,
}

#[derive(Debug, Clone)]
pub struct VerifiedDownloadTask {
    pub url: String,
    pub destination: PathBuf,
    pub checksum: DownloadChecksum,
    pub size: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            limits: shared_transfer_limits(),
            download_concurrency: DEFAULT_CONCURRENCY,
            write_concurrency: DEFAULT_CONCURRENCY,
            retry: DEFAULT_RETRY,
        }
    }
}
