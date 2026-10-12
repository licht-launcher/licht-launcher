use std::path::Path;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::hashes::hex_encode;
use crate::{CoreError, DownloadProgress, Result};

pub(super) fn archive_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(CoreError::CachePath);
    }
    Ok(name)
}

pub(super) async fn download_sha256(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha256: &str,
    size: u64,
    progress: &mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let response = client.get(url).send().await?.error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(destination).await?;
    let mut hasher = Sha256::new();
    let mut done = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        done = done.saturating_add(chunk.len() as u64);
        let _ = progress.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: 1,
            bytes_done: done,
            bytes_total: size,
        });
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);
    let actual = hex_encode(hasher.finalize().as_slice());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        let _ = std::fs::remove_file(destination);
        return Err(CoreError::Sha256Mismatch {
            expected: expected_sha256.to_string(),
            actual,
        });
    }
    Ok(())
}
