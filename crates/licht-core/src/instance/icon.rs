//! Best-effort downloads of instance icons into the launcher's disposable icon cache.

use std::fs;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::Client;

use crate::{CoreError, Result};

pub(crate) fn source_icon_url(instance: &crate::Instance) -> Option<&str> {
    match &instance.installation.as_ref()?.source {
        crate::InstanceSource::Modpack { icon_url, .. } => icon_url.as_deref(),
        _ => None,
    }
}

pub(crate) fn cached_icon_path(dir: &Path, url: Option<&str>) -> Option<PathBuf> {
    let launcher = dir.parent()?.parent()?;
    let key = crate::hashes::sha1_hex(url.unwrap_or(dir.file_name()?.to_str()?).as_bytes());
    Some(
        launcher
            .join("cache")
            .join("icons")
            .join(format!("{key}.png")),
    )
}

pub(crate) fn duplicate_cached_icon(
    source: &Path,
    destination: &Path,
    instance: &crate::Instance,
) -> Result<()> {
    let url = source_icon_url(instance);
    if let (Some(source), Some(destination)) = (
        cached_icon_path(source, url),
        cached_icon_path(destination, url),
    ) && source != destination
        && source.is_file()
    {
        std::fs::create_dir_all(destination.parent().ok_or(CoreError::CachePath)?)?;
        std::fs::copy(source, destination)?;
    }
    Ok(())
}

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

/// Downloads `url` into `cache/icons`. Callers treat failure as non-fatal.
pub async fn download_instance_icon(client: &Client, dir: &Path, url: &str) -> Result<()> {
    if !icon_url_allowed(url) {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "instance icon URL is not allowed",
        )));
    }
    let destination = cached_icon_path(dir, Some(url)).ok_or(CoreError::CachePath)?;
    if destination.is_file() {
        return Ok(());
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
    let launcher = dir
        .parent()
        .and_then(Path::parent)
        .ok_or(CoreError::CachePath)?;
    let _publishing =
        crate::store::layout::initialization_lock(&crate::SharedCache::at(launcher), "icons.lock")
            .await?;
    if !destination.is_file() {
        fs::create_dir_all(destination.parent().ok_or(CoreError::CachePath)?)?;
        let temporary = destination.with_extension("png.part");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, destination)?;
    }
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

    #[tokio::test]
    async fn cached_icons_are_shared_by_url_and_reused_without_network() {
        let root = std::env::temp_dir().join(format!(
            "licht-icon-cache-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let first = root.join("instances").join("First");
        let second = root.join("instances").join("Second");
        let url = "https://cdn.modrinth.com/data/fixture/icon.png";
        let cached = cached_icon_path(&first, Some(url)).expect("cache path");
        assert_eq!(
            cached,
            cached_icon_path(&second, Some(url)).expect("same cache key")
        );
        assert_eq!(
            cached.parent().expect("icons directory"),
            root.join("cache").join("icons")
        );
        fs::create_dir_all(cached.parent().expect("parent")).expect("directory");
        fs::write(&cached, b"cached icon").expect("cached image");
        let client = Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:1").expect("offline proxy"))
            .build()
            .expect("client");
        download_instance_icon(&client, &second, url)
            .await
            .expect("cache hit");
        assert_eq!(fs::read(cached).expect("image"), b"cached icon");
        assert!(!second.join(ICON_FILE).exists());
    }

    #[tokio::test]
    async fn duplication_skips_install_workspace_and_preserves_cached_instance_icons() {
        let root = std::env::temp_dir().join(format!(
            "licht-icon-duplicate-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let instances = root.join("instances");
        let settings = crate::load_settings(&crate::SharedCache::at(&root)).expect("settings");
        let entry = crate::create_instance(
            &instances,
            "Source",
            "1.21",
            &crate::global_launch(&settings),
        )
        .expect("source");
        let source = instances.join(&entry.folder);
        let cached = cached_icon_path(&source, None).expect("icon path");
        fs::create_dir_all(cached.parent().expect("parent")).expect("icons directory");
        fs::write(&cached, b"custom icon").expect("icon");
        fs::create_dir_all(source.join(".install")).expect("workspace");
        fs::write(source.join(".install").join("pack.zip.part"), b"partial").expect("partial");
        fs::create_dir_all(source.join("config").join(".install")).expect("nested user directory");
        fs::write(
            source.join("config").join(".install").join("user.txt"),
            b"user data",
        )
        .expect("nested user file");
        let service = crate::DatabaseService::start(crate::SharedCache::at(&root))
            .await
            .expect("database");
        let copied =
            crate::duplicate_instance(&service.client(), &instances, &entry.folder, "Copy")
                .await
                .expect("duplicate");
        let destination = instances.join(&copied.folder);
        assert!(!destination.join(".install").exists());
        assert_eq!(
            fs::read(destination.join("config").join(".install").join("user.txt"))
                .expect("nested file"),
            b"user data"
        );
        assert_eq!(
            fs::read(copied.icon_path.expect("duplicated cached icon")).expect("icon"),
            b"custom icon"
        );
        assert!(cached.is_file());
        service.shutdown().await.expect("shutdown");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
