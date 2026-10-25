use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use futures_util::TryStreamExt;
use reqwest::StatusCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_util::io::StreamReader;
use tracing::warn;

mod bandwidth;
mod http;
mod limits;
mod progress;
mod queue;
mod types;
pub(crate) use types::SizePolicy;

pub use limits::{TransferLimits, shared_transfer_limits};
pub(crate) use queue::download_verified_all_with_cleanup;
pub use queue::{download_all, download_all_with_control, download_verified_all};
pub use types::{
    DEFAULT_CONCURRENCY, DEFAULT_RETRY, DownloadOptions, DownloadProgress, DownloadTask,
    InstallProgress, Retry, VerifiedDownloadTask,
};

use super::checksum::TransferHasher;
use crate::{CoreError, DownloadChecksum, DownloadControl, Result};
use http::{invalid_response, request, response_total};
use progress::{Attempt, ByteProgress};

const CHUNK_SIZE: usize = 64 * 1024;

/// One mutex per destination so parallel installs cannot corrupt a shared file.
static DESTINATION_LOCKS: LazyLock<StdMutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CancellationCleanup {
    RemovePartial,
    // The job store decides whether another unfinished job still needs these bytes.
    DeferToJob,
}

struct TransferJob<'a> {
    client: &'a reqwest::Client,
    url: &'a str,
    destination: &'a Path,
    checksum: DownloadChecksum,
    published_size: u64,
    size_policy: SizePolicy,
    limits: &'a TransferLimits,
    download_concurrency: usize,
    write_concurrency: usize,
    retry: Retry,
    progress: Option<&'a ByteProgress>,
    control: &'a DownloadControl,
    cancellation_cleanup: CancellationCleanup,
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
        checksum: DownloadChecksum::legacy_sha1(expected_sha1),
        published_size: 0,
        size_policy: SizePolicy::Exact,
        limits: &shared_transfer_limits(),
        download_concurrency: DEFAULT_CONCURRENCY,
        write_concurrency: DEFAULT_CONCURRENCY,
        retry,
        progress: None,
        control,
        cancellation_cleanup: CancellationCleanup::RemovePartial,
    })
    .await
}

async fn download_one(job: TransferJob<'_>) -> Result<()> {
    job.checksum.validate()?;
    if job.size_policy == SizePolicy::Hint && job.checksum.expected().is_none() {
        return Err(invalid_response(
            "estimated download sizes require a checksum",
        ));
    }
    if job.retry.attempts == 0 {
        return Err(CoreError::DownloadAttempts);
    }

    let lock = destination_lock(job.destination);
    let _guard = loop {
        job.control.ready().await?;
        tokio::select! {
            biased;
            error = job.control.interrupted() => {
                if matches!(error, CoreError::DownloadCancelled) {
                    return Err(error);
                }
            }
            guard = lock.lock() => break guard,
        }
    };

    let part = part_path(job.destination)?;
    let result = download_locked(&job, &part).await;
    if matches!(result, Err(CoreError::DownloadCancelled))
        && job.cancellation_cleanup == CancellationCleanup::RemovePartial
    {
        // Only the owner of this destination lock may remove its partial.
        remove_partial(&part).await?;
    }

    result
}

async fn download_locked(job: &TransferJob<'_>, part: &Path) -> Result<()> {
    job.control.ready().await?;

    let mut credited = Attempt {
        progress: job.progress,
        added: 0,
        committed: false,
        known_total: job.published_size,
    };

    if job.checksum.matches_file(job.destination)? {
        let size = if job.published_size == 0 || job.size_policy == SizePolicy::Hint {
            std::fs::metadata(job.destination)?.len()
        } else {
            job.published_size
        };
        credited.set_total(size);
        credited.set(size);
        credited.committed = true;
        return Ok(());
    }

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
                        if matches!(error, CoreError::DownloadCancelled) {
                            return Err(error);
                        }
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
    if job.size_policy == SizePolicy::Hint && offset > credited.known_total {
        credited.set_total(offset);
    }
    credited.set(offset);

    if offset > 0 && job.checksum.matches(&hasher.clone().finish()) {
        credited.set_total(offset);
        credited.set(offset);
        return Ok(());
    }

    let _download = tokio::select! {
        biased;
        error = job.control.interrupted() => return Err(error),
        permit = job.limits.download.acquire(job.download_concurrency) => permit,
    };
    let _activity = job.control.begin_transfer()?;
    let mut response = request(job, offset).await?;

    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && offset > 0 {
        drop(response);
        remove_partial(part).await?;
        offset = 0;
        hasher = job.checksum.hasher();
        credited.set(0);
        response = request(job, 0).await?;
    }

    let response = response.error_for_status()?;
    if response.status() == StatusCode::OK {
        offset = 0;
        hasher = job.checksum.hasher();
        credited.set(0);
    }

    let total = match response_total(&response, offset, job.published_size, job.size_policy) {
        Ok(total) => total,
        Err(error) => {
            remove_partial(part).await?;
            credited.set(0);
            return Err(error);
        }
    };
    if let Some(total) = total {
        credited.set_total(total);
    }
    credited.set(offset);

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
        let requested = total
            .map(|total| {
                total
                    .saturating_sub(offset)
                    .saturating_add(1)
                    .min(CHUNK_SIZE as u64) as usize
            })
            .unwrap_or(CHUNK_SIZE);
        let mut budget = tokio::select! {
            biased;
            error = job.control.interrupted() => return Err(error),
            permit = job.limits.bandwidth.acquire(requested) => permit,
        };
        let read = tokio::select! {
            biased;
            error = job.control.interrupted() => return Err(error),
            read = reader.read(&mut buffer[..requested]) => read?,
        };
        budget.consume(read);
        drop(budget);
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
        if job.size_policy == SizePolicy::Hint && total.is_none() && offset > credited.known_total {
            credited.set_total(offset);
        }
        credited.set(offset);
    }

    file.sync_data().await?;
    drop(file);

    if total.is_some_and(|total| offset != total) {
        return Err(invalid_response("body is shorter than its declared size"));
    }
    if job.size_policy == SizePolicy::Hint {
        credited.set_total(offset);
    }

    if job.checksum.expected().is_none() {
        warn!("download has no checksum; content is not verified");
    } else {
        let actual = hasher.finish();
        if !job.checksum.matches(&actual) {
            remove_partial(part).await?;
            credited.set(0);
            return Err(job.checksum.mismatch(actual));
        }
    }

    Ok(())
}

async fn read_prefix(job: &TransferJob<'_>, part: &Path) -> Result<(u64, TransferHasher)> {
    let mut hasher = job.checksum.hasher();

    // Without a checksum there is no stable content identity across calls.
    if job.checksum.expected().is_none() {
        return Ok((0, hasher));
    }

    let mut file = match tokio::fs::File::open(part).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((0, hasher)),
        Err(error) => return Err(error.into()),
    };

    if job.size_policy == SizePolicy::Exact
        && job.published_size > 0
        && file.metadata().await?.len() > job.published_size
    {
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

async fn remove_partial(part: &Path) -> Result<()> {
    match tokio::fs::remove_file(part).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn remove_partial_if_idle(destination: &Path) -> Result<()> {
    let lock = destination_lock(destination);
    if let Ok(_guard) = lock.try_lock() {
        remove_partial(&part_path(destination)?).await?;
    }

    Ok(())
}

pub(crate) fn destination_lock(destination: &Path) -> Arc<Mutex<()>> {
    let mut locks = DESTINATION_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    Arc::clone(
        locks
            .entry(destination.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
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
