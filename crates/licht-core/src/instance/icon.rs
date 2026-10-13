//! Best-effort download of a per-instance UI icon (`icon.png`).

use std::fs;
use std::path::Path;

use futures_util::StreamExt;
use reqwest::Client;

use crate::{CoreError, Result};

/// File name stored beside `instance.json`.
pub const ICON_FILE: &str = "icon.png";

const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;

/// HTTPS only, no credentials, no explicit port. Hosts are not allowlisted so
/// Modrinth/CurseForge CDNs (and their redirects) can be used for UI art.
pub fn icon_url_allowed(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https" || url.port().is_some() {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    matches!(url.host(), Some(url::Host::Domain(_)))
}

/// Downloads `url` into `dir/icon.png`. Callers treat failure as non-fatal.
pub async fn download_instance_icon(client: &Client, dir: &Path, url: &str) -> Result<()> {
    if !icon_url_allowed(url) {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "instance icon URL is not allowed",
        )));
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(CoreError::from)?
        .error_for_status()
        .map_err(CoreError::from)?;
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(CoreError::from)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_ICON_BYTES {
            return Err(CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "instance icon exceeds size limit",
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "instance icon response was empty",
        )));
    }
    fs::write(dir.join(ICON_FILE), bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_url_rules_are_strict() {
        assert!(icon_url_allowed("https://cdn.modrinth.com/data/x/icon.png"));
        assert!(!icon_url_allowed("http://cdn.modrinth.com/data/x/icon.png"));
        assert!(!icon_url_allowed(
            "https://user:pass@cdn.modrinth.com/x.png"
        ));
        assert!(!icon_url_allowed("https://cdn.modrinth.com:8443/x.png"));
        assert!(!icon_url_allowed("ftp://cdn.modrinth.com/x.png"));
        assert!(!icon_url_allowed("not-a-url"));
    }

    #[tokio::test]
    async fn download_rejects_disallowed_urls_without_writing() {
        let dir = std::env::temp_dir().join(format!("licht-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let client = Client::new();
        let result = download_instance_icon(&client, &dir, "http://example.com/icon.png").await;
        assert!(result.is_err());
        assert!(!dir.join(ICON_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
