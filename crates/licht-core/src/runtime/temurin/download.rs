use std::path::Path;

use crate::{CoreError, DownloadChecksum, InstallationContext, Result, VerifiedDownloadTask};

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
    progress: &InstallationContext,
) -> Result<()> {
    let task = VerifiedDownloadTask {
        url: url.to_string(),
        destination: destination.to_path_buf(),
        checksum: DownloadChecksum::Sha256(expected_sha256.to_string()),
        size,
    };
    progress
        .verified_queue(client, &[task], progress.options()?)
        .await
}
