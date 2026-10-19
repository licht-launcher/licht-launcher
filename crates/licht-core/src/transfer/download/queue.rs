use super::{
    ByteProgress, CancellationCleanup, DownloadOptions, DownloadProgress, DownloadTask, Retry,
    TransferJob, TransferLimits, VerifiedDownloadTask, download_one,
};
use crate::{CoreError, DownloadChecksum, DownloadControl, Result};
use futures_util::{StreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::mpsc;

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
    let verified: Vec<_> = tasks
        .iter()
        .map(|task| VerifiedDownloadTask {
            url: task.url.clone(),
            destination: task.destination.clone(),
            checksum: DownloadChecksum::legacy_sha1(&task.sha1),
            size: task.size,
        })
        .collect();
    download_verified_all(client, &verified, options, control, progress).await
}

pub async fn download_verified_all(
    client: &reqwest::Client,
    tasks: &[VerifiedDownloadTask],
    options: &DownloadOptions,
    control: &DownloadControl,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    download_verified_all_with_cleanup(
        client,
        tasks,
        options,
        control,
        progress,
        CancellationCleanup::RemovePartial,
    )
    .await
}

pub(crate) async fn download_verified_all_with_cleanup(
    client: &reqwest::Client,
    tasks: &[VerifiedDownloadTask],
    options: &DownloadOptions,
    control: &DownloadControl,
    progress: mpsc::Sender<DownloadProgress>,
    cancellation_cleanup: CancellationCleanup,
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
        total: AtomicU64::new(
            tasks
                .iter()
                .fold(0, |sum, task| sum.saturating_add(task.size)),
        ),
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
            checksum: task.checksum.clone(),
            published_size: task.size,
            limits: &options.limits,
            download_concurrency: options.download_concurrency,
            write_concurrency: options.write_concurrency,
            retry: options.retry,
            progress: Some(&bytes),
            control,
            cancellation_cleanup,
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
