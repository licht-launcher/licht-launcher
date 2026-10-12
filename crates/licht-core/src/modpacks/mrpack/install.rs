use std::fs;
use std::path::Path;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;

use super::MODPACK_INDEX_FILE;
use super::index::{MRPACK_DOWNLOAD_HOSTS, index_files};
use crate::download::{DEFAULT_RETRY, DownloadProgress, download_all};
use crate::modpacks::file_hashes::{ensure_sha512, prepare_download};
use crate::modpacks::secure_download::allowlisted_download_client;
use crate::settings::{download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};

/// Downloads every client-required file from [`MODPACK_INDEX_FILE`] into the instance.
///
/// No-op when the index is missing (vanilla / CurseForge without mrpack files).
/// A redirect is followed only when that hop is HTTPS and the host is
/// `cdn.modrinth.com`.
pub async fn install_mrpack_files(
    game: &SharedCache,
    settings: &SharedCache,
    instance_dir: &Path,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let index_path = instance_dir.join(MODPACK_INDEX_FILE);
    if !index_path.is_file() {
        return Ok(());
    }
    let body = fs::read_to_string(&index_path)?;
    let index: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    let files = index_files(&index, instance_dir)?;
    let mut pending = Vec::new();
    for file in &files {
        if !prepare_download(&file.task.destination, &file.task.sha1, &file.sha512)? {
            continue;
        }
        if let Some(parent) = file.task.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        pending.push(file.task.clone());
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
    download_all(
        &client,
        &pending,
        shared_transfer_limits(),
        download_concurrency(game, settings)?,
        write_concurrency(game, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await?;
    for file in &files {
        ensure_sha512(&file.task.destination, &file.sha512)?;
    }
    Ok(())
}

fn mrpack_download_client() -> Result<Client> {
    allowlisted_download_client(
        MRPACK_DOWNLOAD_HOSTS,
        "modpack redirect target is not allowed",
        "too many modpack redirects",
    )
}
