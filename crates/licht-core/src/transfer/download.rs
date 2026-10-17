use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::Duration;

use futures_util::{StreamExt, TryStreamExt, stream::FuturesUnordered};
use reqwest::StatusCode;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_RANGE, RANGE};
use serde::Serialize;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_util::io::StreamReader;
use tracing::warn;
use ts_rs::TS;

use crate::hashes::{file_matches_sha1, hex_encode};
use crate::{CoreError, DownloadControl, Result};

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
    control: &'a DownloadControl,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub limits: Arc<TransferLimits>,
    pub download_concurrency: usize,
    pub write_concurrency: usize,
    pub retry: Retry,
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

pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha1: &str,
    retry: Retry,
) -> Result<()> {
    download_file_with_control(
        client,
        url,
        destination,
        expected_sha1,
        retry,
        &DownloadControl::new(),
    )
    .await
}

/// Pausing preserves the partial and keeps this future pending until resumed.
/// Dropping the future preserves its partial for a new call with the same checksum.
/// Unverified downloads restart instead of reusing partial bytes. A paused transfer
/// retains its destination lock, but releases its network and write permits.
pub async fn download_file_with_control(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha1: &str,
    retry: Retry,
    control: &DownloadControl,
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
        control,
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
    let options = DownloadOptions {
        limits,
        download_concurrency,
        write_concurrency,
        retry,
    };
    download_all_with_control(client, tasks, &options, &DownloadControl::new(), progress).await
}

/// A dropped queue drops its in-flight transfers instead of leaving detached tasks.
/// Cancellation waits for owned partials to be removed and preserves complete files.
pub async fn download_all_with_control(
    client: &reqwest::Client,
    tasks: &[DownloadTask],
    options: &DownloadOptions,
    control: &DownloadControl,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    control.ready().await?;
    if tasks.is_empty() {
        return Ok(());
    }
    if options.download_concurrency == 0 {
        return Err(CoreError::DownloadConcurrency);
    }
    if options.write_concurrency == 0 {
        return Err(CoreError::WriteConcurrency);
    }

    let total = u32::try_from(tasks.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "too many downloads"))?;
    let bytes = ByteProgress {
        done: AtomicU64::new(0),
        total: tasks
            .iter()
            .fold(0, |sum, task| sum.saturating_add(task.size)),
        files: total,
        sink: progress.clone(),
    };
    let _ = bytes.sink.try_send(bytes.snapshot(0, 0));
    let mut pending = FuturesUnordered::new();
    for task in tasks {
        pending.push(download_one(TransferJob {
            client,
            url: &task.url,
            destination: &task.destination,
            expected_sha1: &task.sha1,
            published_size: task.size,
            limits: &options.limits,
            download_concurrency: options.download_concurrency,
            write_concurrency: options.write_concurrency,
            retry: options.retry,
            progress: Some(&bytes),
            control,
        }));
    }

    let mut finished = 0;
    let mut failed = 0;
    let mut first_error = None;
    let mut cancelled = false;
    while let Some(result) = pending.next().await {
        match result {
            Ok(()) => finished += 1,
            Err(CoreError::DownloadCancelled) => cancelled = true,
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
                failed += 1;
            }
        }
        let _ = progress.try_send(bytes.snapshot(finished, failed));
    }

    if cancelled {
        return Err(CoreError::DownloadCancelled);
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

    let lock = destination_lock(job.destination);
    let _guard = loop {
        job.control.ready().await?;
        tokio::select! {
            biased;
            error = job.control.interrupted() => {
                if matches!(error, CoreError::DownloadCancelled) { return Err(error); }
            }
            guard = lock.lock() => break guard,
        }
    };
    let part = part_path(job.destination)?;
    let result = download_locked(&job, &part).await;
    if matches!(result, Err(CoreError::DownloadCancelled)) {
        // Only the owner of this destination lock may remove its partial.
        remove_partial(&part).await?;
    }
    result
}

async fn download_locked(job: &TransferJob<'_>, part: &Path) -> Result<()> {
    job.control.ready().await?;
    if file_matches_sha1(job.destination, job.expected_sha1)? {
        if let Some(progress) = job.progress {
            progress.add(job.published_size);
        }
        return Ok(());
    }
    let mut credited = Attempt {
        progress: job.progress,
        added: 0,
        committed: false,
    };
    let mut backoff = job.retry.initial_backoff;
    let mut attempt = 1;
    loop {
        job.control.ready().await?;
        let result = async {
            transfer_partial(job, part, &mut credited).await?;
            job.control.ready().await?;
            let _write = tokio::select! {
                biased;
                error = job.control.interrupted() => return Err(error),
                permit = job.limits.write.acquire(job.write_concurrency) => permit,
            };
            if tokio::fs::try_exists(job.destination).await? {
                tokio::fs::remove_file(job.destination).await?;
            }
            tokio::fs::rename(part, job.destination).await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                credited.committed = true;
                return Ok(());
            }
            Err(CoreError::DownloadPaused) => continue,
            Err(CoreError::DownloadCancelled) => return Err(CoreError::DownloadCancelled),
            Err(error) => {
                if attempt == job.retry.attempts {
                    return Err(error);
                }
                warn!(attempt, "download attempt failed");
                tokio::select! {
                    biased;
                    error = job.control.interrupted() => {
                        if matches!(error, CoreError::DownloadCancelled) { return Err(error); }
                    }
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = backoff.saturating_mul(2);
                attempt += 1;
            }
        }
    }
}

async fn transfer_partial(
    job: &TransferJob<'_>,
    part: &Path,
    credited: &mut Attempt<'_>,
) -> Result<()> {
    let (mut offset, mut hasher) = read_prefix(job, part).await?;
    credited.set(offset);
    if offset > 0
        && hex_encode(hasher.clone().finalize().as_slice()).eq_ignore_ascii_case(job.expected_sha1)
    {
        return Ok(());
    }
    let _download = tokio::select! {
        biased;
        error = job.control.interrupted() => return Err(error),
        permit = job.limits.download.acquire(job.download_concurrency) => permit,
    };
    let mut response = request(job, offset).await?;
    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && offset > 0 {
        drop(response);
        remove_partial(part).await?;
        offset = 0;
        hasher = Sha1::new();
        credited.set(0);
        response = request(job, 0).await?;
    }
    let response = response.error_for_status()?;
    if response.status() == StatusCode::OK {
        offset = 0;
        hasher = Sha1::new();
        credited.set(0);
    }
    let total = match response_total(&response, offset, job.published_size) {
        Ok(total) => total,
        Err(error) => {
            remove_partial(part).await?;
            credited.set(0);
            return Err(error);
        }
    };
    if let Some(parent) = part.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(offset > 0)
        .truncate(offset == 0)
        .open(part)
        .await?;
    let stream = response.bytes_stream().map_err(std::io::Error::other);
    let mut reader = StreamReader::new(stream);
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    loop {
        let read = tokio::select! {
            biased;
            error = job.control.interrupted() => return Err(error),
            read = reader.read(&mut buffer) => read?,
        };
        if read == 0 {
            break;
        }
        if total.is_some_and(|total| offset.saturating_add(read as u64) > total) {
            drop(file);
            remove_partial(part).await?;
            credited.set(0);
            return Err(invalid_response("body exceeds its declared size"));
        }
        {
            let _write = tokio::select! {
                biased;
                error = job.control.interrupted() => return Err(error),
                permit = job.limits.write.acquire(job.write_concurrency) => permit,
            };
            // Finish this bounded write before observing pause/cancel, so the
            // next Range offset describes bytes actually committed to the file.
            file.write_all(&buffer[..read]).await?;
            file.flush().await?;
        }
        hasher.update(&buffer[..read]);
        offset += read as u64;
        credited.set(offset);
    }
    file.sync_data().await?;
    drop(file);
    if total.is_some_and(|total| offset != total) {
        return Err(invalid_response("body is shorter than its declared size"));
    }
    if job.expected_sha1.is_empty() {
        warn!("download has no SHA1; content is not verified");
    } else {
        let actual = hex_encode(hasher.finalize().as_slice());
        if !actual.eq_ignore_ascii_case(job.expected_sha1) {
            remove_partial(part).await?;
            credited.set(0);
            return Err(CoreError::Sha1Mismatch {
                expected: job.expected_sha1.to_string(),
                actual,
            });
        }
    }
    Ok(())
}

async fn read_prefix(job: &TransferJob<'_>, part: &Path) -> Result<(u64, Sha1)> {
    let mut hasher = Sha1::new();
    // Without a checksum there is no stable content identity across calls.
    if job.expected_sha1.is_empty() {
        return Ok((0, hasher));
    }
    let mut file = match tokio::fs::File::open(part).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((0, hasher)),
        Err(error) => return Err(error.into()),
    };
    if job.published_size > 0 && file.metadata().await?.len() > job.published_size {
        drop(file);
        remove_partial(part).await?;
        return Ok((0, hasher));
    }
    let mut offset = 0;
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    loop {
        let count = tokio::select! {
            biased;
            error = job.control.interrupted() => return Err(error),
            count = file.read(&mut buffer) => count?,
        };
        if count == 0 {
            return Ok((offset, hasher));
        }
        hasher.update(&buffer[..count]);
        offset += count as u64;
    }
}

async fn request(job: &TransferJob<'_>, offset: u64) -> Result<reqwest::Response> {
    let mut request = job.client.get(job.url).header(ACCEPT_ENCODING, "identity");
    if offset > 0 {
        request = request.header(RANGE, format!("bytes={offset}-"));
    }
    tokio::select! {
        biased;
        error = job.control.interrupted() => Err(error),
        response = request.send() => Ok(response?),
    }
}

fn response_total(
    response: &reqwest::Response,
    offset: u64,
    published_size: u64,
) -> Result<Option<u64>> {
    if let Some(encoding) = response.headers().get(CONTENT_ENCODING)
        && !encoding.as_bytes().eq_ignore_ascii_case(b"identity")
    {
        return Err(invalid_response("encoded bodies cannot be resumed"));
    }
    let total = match response.status() {
        StatusCode::OK => response.content_length(),
        StatusCode::PARTIAL_CONTENT => {
            let range = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("bytes "))
                .ok_or_else(|| invalid_response("missing or invalid Content-Range"))?;
            let (bounds, total) = range
                .split_once('/')
                .ok_or_else(|| invalid_response("invalid range total"))?;
            let (start, end) = bounds
                .split_once('-')
                .ok_or_else(|| invalid_response("invalid range bounds"))?;
            let parse = |value: &str| {
                value
                    .parse::<u64>()
                    .map_err(|_| invalid_response("invalid range number"))
            };
            let (start, end, total) = (parse(start)?, parse(end)?, parse(total)?);
            if start != offset || total <= start || end.checked_add(1) != Some(total) {
                return Err(invalid_response(
                    "range does not match the requested suffix",
                ));
            }
            if response
                .content_length()
                .is_some_and(|length| length != total - start)
            {
                return Err(invalid_response(
                    "range length does not match Content-Length",
                ));
            }
            Some(total)
        }
        _ => return Err(invalid_response("expected a full or partial file response")),
    };
    if published_size > 0 {
        if total.is_some_and(|total| total != published_size) {
            return Err(invalid_response(
                "response size does not match published size",
            ));
        }
        return Ok(Some(published_size));
    }
    Ok(total)
}

fn invalid_response(detail: &'static str) -> CoreError {
    CoreError::DownloadResponse { detail }
}

async fn remove_partial(part: &Path) -> Result<()> {
    match tokio::fs::remove_file(part).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
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

/// Contribution of one destination, retained across pause/retry and removed on failure.
struct Attempt<'a> {
    progress: Option<&'a ByteProgress>,
    added: u64,
    committed: bool,
}

impl Attempt<'_> {
    fn set(&mut self, bytes: u64) {
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
