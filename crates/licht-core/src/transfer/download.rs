use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::Duration;

use futures_util::TryStreamExt;
use serde::Serialize;
use sha1::{Digest, Sha1};
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_util::io::StreamReader;
use tracing::warn;
use ts_rs::TS;

use crate::{CoreError, Result};

const CHUNK_SIZE: usize = 64 * 1024;

pub const DEFAULT_CONCURRENCY: usize = 8;

pub const DEFAULT_RETRY: Retry = Retry {
    attempts: 3,
    initial_backoff: Duration::from_millis(200),
};

/// One mutex per destination so parallel installs cannot corrupt a shared file.
static DESTINATION_LOCKS: LazyLock<StdMutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
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

struct TransferJob<'a> {
    client: &'a reqwest::Client,
    url: &'a str,
    destination: &'a Path,
    expected_sha1: &'a str,
    published_size: u64,
    limits: &'a TransferLimits,
    download_concurrency: usize,
    write_concurrency: usize,
    retry: Retry,
    progress: Option<&'a ByteProgress>,
}

pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha1: &str,
    retry: Retry,
) -> Result<()> {
    download_one(TransferJob {
        client,
        url,
        destination,
        expected_sha1,
        published_size: 0,
        limits: &shared_transfer_limits(),
        download_concurrency: DEFAULT_CONCURRENCY,
        write_concurrency: DEFAULT_CONCURRENCY,
        retry,
        progress: None,
    })
    .await
}

pub async fn download_all(
    client: &reqwest::Client,
    tasks: &[DownloadTask],
    limits: Arc<TransferLimits>,
    download_concurrency: usize,
    write_concurrency: usize,
    retry: Retry,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    if tasks.is_empty() {
        return Ok(());
    }
    if download_concurrency == 0 {
        return Err(CoreError::DownloadConcurrency);
    }
    if write_concurrency == 0 {
        return Err(CoreError::WriteConcurrency);
    }

    let total = u32::try_from(tasks.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "too many downloads"))?;
    let bytes = Arc::new(ByteProgress {
        done: AtomicU64::new(0),
        total: tasks
            .iter()
            .fold(0, |sum, task| sum.saturating_add(task.size)),
        files: total,
        sink: progress.clone(),
    });
    let _ = bytes.sink.try_send(bytes.snapshot(0, 0));
    let (results, mut incoming) = mpsc::channel(tasks.len());

    for task in tasks {
        let results = results.clone();
        let client = client.clone();
        let limits = Arc::clone(&limits);
        let bytes = Arc::clone(&bytes);
        let url = task.url.clone();
        let destination = task.destination.clone();
        let sha1 = task.sha1.clone();
        let size = task.size;
        tokio::spawn(async move {
            let result = download_one(TransferJob {
                client: &client,
                url: &url,
                destination: &destination,
                expected_sha1: &sha1,
                published_size: size,
                limits: &limits,
                download_concurrency,
                write_concurrency,
                retry,
                progress: Some(&bytes),
            })
            .await;
            let _ = results.send(result).await;
        });
    }
    drop(results);

    let mut finished = 0;
    let mut failed = 0;
    let mut first_error = None;
    while let Some(result) = incoming.recv().await {
        match result {
            Ok(()) => finished += 1,
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
                failed += 1;
            }
        }
        let _ = progress.try_send(bytes.snapshot(finished, failed));
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

async fn download_one(job: TransferJob<'_>) -> Result<()> {
    if job.retry.attempts == 0 {
        return Err(CoreError::DownloadAttempts);
    }

    {
        let lock = destination_lock(job.destination);
        let _guard = lock.lock().await;
        if file_matches(job.destination, job.expected_sha1).await? {
            if let Some(progress) = job.progress {
                progress.add(job.published_size);
            }
            return Ok(());
        }
    }

    let mut backoff = job.retry.initial_backoff;
    let mut attempt = 1;
    loop {
        let fetched = {
            let _download = job.limits.download.acquire(job.download_concurrency).await;
            match fetch_bytes(job.client, job.url, job.progress).await {
                Ok(body) => body,
                Err(error) => {
                    if attempt == job.retry.attempts {
                        return Err(error);
                    }
                    warn!(attempt, url = job.url, "download attempt failed");
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2);
                    attempt += 1;
                    continue;
                }
            }
        };

        // Fabric/Quilt Maven entries sometimes omit hashes. Download them, but
        // never treat a missing hash as a successful check.
        if job.expected_sha1.is_empty() {
            warn!(
                url = job.url,
                "download has no SHA1; content is not verified"
            );
        } else {
            let actual = hex_encode(Sha1::digest(&fetched).as_slice());
            if !actual.eq_ignore_ascii_case(job.expected_sha1) {
                if let Some(progress) = job.progress {
                    progress.subtract(fetched.len() as u64);
                }
                let error = CoreError::Sha1Mismatch {
                    expected: job.expected_sha1.to_string(),
                    actual,
                };
                if attempt == job.retry.attempts {
                    return Err(error);
                }
                warn!(attempt, url = job.url, "download attempt failed");
                tokio::time::sleep(backoff).await;
                backoff = backoff.saturating_mul(2);
                attempt += 1;
                continue;
            }
        }

        let _write = job.limits.write.acquire(job.write_concurrency).await;
        let lock = destination_lock(job.destination);
        let _guard = lock.lock().await;
        if file_matches(job.destination, job.expected_sha1).await? {
            return Ok(());
        }
        match write_bytes(job.destination, &fetched).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                if let Some(progress) = job.progress {
                    progress.subtract(fetched.len() as u64);
                }
                if attempt == job.retry.attempts {
                    return Err(error);
                }
                warn!(attempt, url = job.url, "download attempt failed");
                tokio::time::sleep(backoff).await;
                backoff = backoff.saturating_mul(2);
                attempt += 1;
            }
        }
    }
}

async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    progress: Option<&ByteProgress>,
) -> Result<Vec<u8>> {
    let mut attempt = progress.map(|progress| Attempt {
        progress,
        added: 0,
        committed: false,
    });
    let response = client.get(url).send().await?.error_for_status()?;
    let stream = response.bytes_stream().map_err(std::io::Error::other);
    let mut reader = StreamReader::new(stream);
    let mut body = Vec::new();
    let mut buffer = vec![0_u8; CHUNK_SIZE];

    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
        if let Some(attempt) = attempt.as_mut() {
            attempt.add(read as u64);
        }
    }
    if let Some(attempt) = attempt.as_mut() {
        attempt.commit();
    }
    Ok(body)
}

async fn write_bytes(destination: &Path, body: &[u8]) -> Result<()> {
    let part = part_path(destination)?;
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&part, body).await?;
    if tokio::fs::try_exists(destination).await? {
        tokio::fs::remove_file(destination).await?;
    }
    match tokio::fs::rename(&part, destination).await {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = tokio::fs::remove_file(&part).await;
            Err(error.into())
        }
    }
}

fn destination_lock(destination: &Path) -> Arc<Mutex<()>> {
    let mut locks = DESTINATION_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Arc::clone(
        locks
            .entry(destination.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

async fn file_matches(path: &Path, expected_sha1: &str) -> Result<bool> {
    // Existence is not verification. Callers that omit a hash re-download.
    if expected_sha1.is_empty() {
        return Ok(false);
    }
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha1::new();
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(hasher.finalize().as_slice()).eq_ignore_ascii_case(expected_sha1))
}

struct ByteProgress {
    done: AtomicU64,
    total: u64,
    files: u32,
    sink: mpsc::Sender<DownloadProgress>,
}

impl ByteProgress {
    fn add(&self, bytes: u64) {
        let done = self.done.fetch_add(bytes, Ordering::Relaxed) + bytes;
        let _ = self.sink.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: self.files,
            bytes_done: done,
            bytes_total: self.total,
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
            bytes_total: self.total,
        });
    }

    fn snapshot(&self, finished: u32, failed: u32) -> DownloadProgress {
        DownloadProgress {
            finished,
            failed,
            total: self.files,
            bytes_done: self.done.load(Ordering::Relaxed),
            bytes_total: self.total,
        }
    }
}

/// Bytes of one attempt. A failed attempt is removed again so a retry does not count twice.
struct Attempt<'a> {
    progress: &'a ByteProgress,
    added: u64,
    committed: bool,
}

impl Attempt<'_> {
    fn add(&mut self, bytes: u64) {
        self.added = self.added.saturating_add(bytes);
        self.progress.add(bytes);
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if self.committed || self.added == 0 {
            return;
        }
        self.progress.subtract(self.added);
    }
}

fn part_path(destination: &Path) -> Result<PathBuf> {
    let name = destination.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "download destination has no file name",
        )
    })?;
    let mut part_name = name.to_os_string();
    part_name.push(".part");
    Ok(destination.with_file_name(part_name))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
