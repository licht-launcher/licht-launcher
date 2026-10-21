use std::fs;
use std::path::Path;

use super::index::MRPACK_DOWNLOAD_HOSTS;
use crate::download::{DEFAULT_RETRY, DownloadProgress};
use crate::modpacks::file_hashes::{ensure_sha512, prepare_download};
use crate::modpacks::secure_download::allowlisted_download_client;
use crate::settings::{download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};
use reqwest::Client;

/// Downloads every client-required file from the stored Modrinth index into the instance.
///
/// No-op when no files are stored (vanilla / CurseForge without mrpack files).
/// A redirect is followed only when that hop is HTTPS and the host is
/// `cdn.modrinth.com`.
pub async fn install_mrpack_files(
    database: &crate::DatabaseClient,
    game: &SharedCache,
    settings: &SharedCache,
    instance_dir: &Path,
    progress: impl Into<crate::InstallationContext>,
) -> Result<()> {
    let progress = progress.into().component(crate::DownloadComponent::Modpack);
    let instances = instance_dir.parent().ok_or(CoreError::CachePath)?;
    let folder = instance_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CoreError::InstanceName)?;
    if database.modpack_header(instances, folder).await?.is_none() {
        return Ok(());
    }
    let files = database.modpack_files(instances, folder).await?;
    if files.is_empty() {
        return Ok(());
    }
    progress.stage(crate::DownloadPhase::Verifying).await?;
    let mut pending = Vec::new();
    for file in &files {
        let task = file.task(instance_dir)?;
        if !prepare_download(
            &task.destination,
            &task.sha1,
            &file.sha512,
            &progress.control,
        )
        .await?
        {
            continue;
        }
        if let Some(parent) = task.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        pending.push(task);
    }
    if pending.is_empty() {
        let _ = progress.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        });
        return Ok(());
    }
    // The caller's client would follow any redirect. This one checks every hop.
    let client = mrpack_download_client()?;
    progress
        .queue(
            &client,
            &pending,
            crate::DownloadOptions {
                limits: shared_transfer_limits(),
                download_concurrency: download_concurrency(game, settings)?,
                write_concurrency: write_concurrency(game, settings)?,
                retry: DEFAULT_RETRY,
            },
        )
        .await?;
    progress.stage(crate::DownloadPhase::Verifying).await?;
    for file in &files {
        ensure_sha512(
            &file.task(instance_dir)?.destination,
            &file.sha512,
            &progress.control,
        )
        .await?;
    }
    Ok(())
}

pub(crate) fn mrpack_download_client() -> Result<Client> {
    allowlisted_download_client(
        MRPACK_DOWNLOAD_HOSTS,
        "modpack redirect target is not allowed",
        "too many modpack redirects",
    )
}
