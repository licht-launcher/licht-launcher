use super::DownloadProgress;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

pub(super) struct ByteProgress {
    pub(super) done: AtomicU64,
    pub(super) total: AtomicU64,
    pub(super) files: u32,
    pub(super) sink: mpsc::Sender<DownloadProgress>,
}
impl ByteProgress {
    pub(super) fn add(&self, bytes: u64) {
        let done = self.done.fetch_add(bytes, Ordering::Relaxed) + bytes;
        let _ = self.sink.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: self.files,
            bytes_done: done,
            bytes_total: self.total.load(Ordering::Relaxed),
        });
    }
    fn subtract(&self, bytes: u64) {
        if bytes == 0 {
            return;
        }
        let previous = self.done.fetch_sub(bytes, Ordering::Relaxed);
        let done = previous.saturating_sub(bytes);
        let _ = self.sink.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: self.files,
            bytes_done: done,
            bytes_total: self.total.load(Ordering::Relaxed),
        });
    }
    pub(super) fn snapshot(&self, finished: u32, failed: u32) -> DownloadProgress {
        DownloadProgress {
            finished,
            failed,
            total: self.files,
            bytes_done: self.done.load(Ordering::Relaxed),
            bytes_total: self.total.load(Ordering::Relaxed),
        }
    }
}

/// Contribution of one destination, retained across pause/retry and removed on failure.
pub(super) struct Attempt<'a> {
    pub(super) progress: Option<&'a ByteProgress>,
    pub(super) added: u64,
    pub(super) committed: bool,
    pub(super) known_total: u64,
}
impl Attempt<'_> {
    pub(super) fn set_total(&mut self, total: u64) {
        if let Some(progress) = self.progress {
            if total >= self.known_total {
                progress
                    .total
                    .fetch_add(total - self.known_total, Ordering::Relaxed);
            } else {
                progress
                    .total
                    .fetch_sub(self.known_total - total, Ordering::Relaxed);
            }
        }
        self.known_total = total;
    }
    pub(super) fn set(&mut self, bytes: u64) {
        if let Some(progress) = self.progress {
            if bytes >= self.added {
                progress.add(bytes - self.added);
            } else {
                progress.subtract(self.added - bytes);
            }
        }
        self.added = bytes;
    }
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if self.committed || self.added == 0 {
            return;
        }
        if let Some(progress) = self.progress {
            progress.subtract(self.added);
        }
    }
}
