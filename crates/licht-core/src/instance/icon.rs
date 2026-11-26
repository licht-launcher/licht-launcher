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

/// Legacy icon file name inside the instance directory.
pub const ICON_FILE: &str = "icon.png";

const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;

const CUSTOM_ICON_PREFIX: &str = "icon-";
const CUSTOM_ICON_SUFFIX: &str = ".png";

/// Where an icon the user picked is kept. The name follows the image's content, so a new
/// image gets a new path and the webview cannot show a stale copy of the old one.
pub(crate) fn custom_icon_path(dir: &Path, bytes: &[u8]) -> PathBuf {
    let hash = crate::hashes::sha1_hex(bytes);
    dir.join(format!(
        "{CUSTOM_ICON_PREFIX}{}{CUSTOM_ICON_SUFFIX}",
        &hash[..16]
    ))
}

fn is_custom_icon_name(name: &str) -> bool {
    name.strip_prefix(CUSTOM_ICON_PREFIX)
        .and_then(|rest| rest.strip_suffix(CUSTOM_ICON_SUFFIX))
        .is_some_and(|hash| hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Icons the user picked for this instance, newest first; normally at most one.
pub(crate) fn custom_icons(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut icons = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name().to_str().is_some_and(is_custom_icon_name) && entry.path().is_file() {
            icons.push((entry.metadata()?.modified()?, entry.path()));
        }
    }
    // An old icon that could not be removed must not hide the one picked after it.
    icons.sort_by(|left, right| right.cmp(left));
    Ok(icons.into_iter().map(|(_, path)| path).collect())
}

/// Removes the icons the user picked, except `keep`.
pub(crate) fn remove_custom_icons(dir: &Path, keep: Option<&Path>) -> Result<()> {
    for icon in custom_icons(dir)? {
        if Some(icon.as_path()) != keep {
            fs::remove_file(icon)?;
        }
    }
    Ok(())
}

/// Reads a user-picked image, accepting only formats the webview can show.
pub(crate) fn read_custom_icon(source: &Path) -> Result<Vec<u8>> {
    if fs::metadata(source)?.len() > MAX_ICON_BYTES as u64 {
        return Err(CoreError::InstanceIconSize);
    }
    let bytes = fs::read(source)?;
    if bytes.len() > MAX_ICON_BYTES {
        return Err(CoreError::InstanceIconSize);
    }
    if !is_supported_image(&bytes) {
        return Err(CoreError::InstanceIconImage);
    }
    Ok(bytes)
}

fn is_supported_image(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
}

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
    use licht_testkit::temp::TempDir;

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

    #[test]
    fn custom_icons_accept_only_supported_images_within_the_size_limit() {
        let temp = TempDir::new("icon-custom");
        let dir = temp.path();
        let cases: [(&str, &[u8]); 5] = [
            ("icon.png", b"\x89PNG\r\n\x1a\nrest"),
            ("icon.jpg", &[0xFF, 0xD8, 0xFF, 0xE0]),
            ("icon.gif", b"GIF89a rest"),
            ("icon.webp", b"RIFF\0\0\0\0WEBPrest"),
            ("icon.txt", b"not an image"),
        ];
        for (name, bytes) in cases {
            fs::write(dir.join(name), bytes).expect("image");
        }
        for name in ["icon.png", "icon.jpg", "icon.gif", "icon.webp"] {
            assert!(read_custom_icon(&dir.join(name)).is_ok(), "{name}");
        }
        assert!(matches!(
            read_custom_icon(&dir.join("icon.txt")),
            Err(CoreError::InstanceIconImage)
        ));
        let mut large = b"\x89PNG\r\n\x1a\n".to_vec();
        large.resize(MAX_ICON_BYTES + 1, 0);
        fs::write(dir.join("large.png"), large).expect("large image");
        assert!(matches!(
            read_custom_icon(&dir.join("large.png")),
            Err(CoreError::InstanceIconSize)
        ));
    }

    #[tokio::test]
    async fn a_picked_icon_replaces_the_cached_one_until_it_is_reset() {
        let temp = TempDir::new("icon-pick");
        let root = temp.path().to_path_buf();
        let instances = root.join("instances");
        let settings = crate::settings::load_settings_at(&crate::SharedCache::at(&root))
            .await
            .expect("settings");
        let service = crate::DatabaseService::start(
            crate::Database::open(&crate::SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let database = service.client();
        let entry = crate::create_instance(
            &database,
            &instances,
            "Survival",
            "1.21",
            &crate::global_launch(&settings),
        )
        .await
        .expect("instance");
        let dir = instances.join(&entry.folder);
        let cached = cached_icon_path(&dir, None).expect("icon path");
        fs::create_dir_all(cached.parent().expect("parent")).expect("icons directory");
        fs::write(&cached, b"cached icon").expect("cached icon");
        let picked = root.join("picked.gif");
        fs::write(&picked, b"GIF89a picked").expect("picked image");
        let rejected = root.join("notes.txt");
        fs::write(&rejected, b"not an image").expect("text file");

        assert!(matches!(
            crate::set_instance_icon(&database, &instances, &entry.folder, &rejected).await,
            Err(CoreError::InstanceIconImage)
        ));
        assert!(custom_icons(&dir).expect("icons").is_empty());

        let changed = crate::set_instance_icon(&database, &instances, &entry.folder, &picked)
            .await
            .expect("set icon");
        assert!(changed.custom_icon);
        let shown = changed.icon_path.expect("picked icon");
        assert_eq!(fs::read(&shown).expect("icon"), b"GIF89a picked");
        assert_eq!(
            crate::instance_entry(&database, &dir, &entry.folder)
                .await
                .expect("entry")
                .icon_path,
            Some(shown.clone())
        );

        // Another image gets another path, so the webview loads it instead of a stale copy.
        let other = root.join("other.png");
        fs::write(&other, b"\x89PNG\r\n\x1a\nother").expect("other image");
        let replaced = crate::set_instance_icon(&database, &instances, &entry.folder, &other)
            .await
            .expect("replace icon");
        let replaced_path = replaced.icon_path.expect("replaced icon");
        assert_ne!(replaced_path, shown);
        assert_eq!(
            custom_icons(&dir).expect("icons"),
            vec![PathBuf::from(&replaced_path)]
        );

        let reset = crate::reset_instance_icon(&database, &instances, &entry.folder)
            .await
            .expect("reset icon");
        assert!(!reset.custom_icon);
        assert_eq!(reset.icon_path, Some(cached.to_string_lossy().into_owned()));
        assert!(custom_icons(&dir).expect("icons").is_empty());
        crate::reset_instance_icon(&database, &instances, &entry.folder)
            .await
            .expect("resetting twice");
        service.shutdown().await.expect("shutdown");
    }

    #[test]
    fn the_newest_picked_icon_wins_over_one_left_behind() {
        let temp = TempDir::new("icon-newest");
        let dir = temp.path();
        let older = custom_icon_path(dir, b"older");
        let newer = custom_icon_path(dir, b"newer");
        fs::write(&older, b"older").expect("older icon");
        fs::write(&newer, b"newer").expect("newer icon");
        fs::write(dir.join("icon-notes.png"), b"not a picked icon").expect("other file");
        let now = std::time::SystemTime::now();
        for (path, age) in [(&older, 60), (&newer, 0)] {
            fs::File::options()
                .write(true)
                .open(path)
                .expect("icon file")
                .set_modified(now - std::time::Duration::from_secs(age))
                .expect("modified time");
        }
        assert_eq!(custom_icons(dir).expect("icons"), vec![newer, older]);
    }

    #[tokio::test]
    async fn a_failed_icon_change_leaves_no_partial_file() {
        let temp = TempDir::new("icon-failed");
        let root = temp.path().to_path_buf();
        let instances = root.join("instances");
        let settings = crate::settings::load_settings_at(&crate::SharedCache::at(&root))
            .await
            .expect("settings");
        let service = crate::DatabaseService::start(
            crate::Database::open(&crate::SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let database = service.client();
        let entry = crate::create_instance(
            &database,
            &instances,
            "Survival",
            "1.21",
            &crate::global_launch(&settings),
        )
        .await
        .expect("instance");
        let dir = instances.join(&entry.folder);
        let bytes = b"GIF89a picked";
        let picked = root.join("picked.gif");
        fs::write(&picked, bytes).expect("picked image");
        // A directory where the icon goes makes moving the written image into place fail.
        let destination = custom_icon_path(&dir, bytes);
        fs::create_dir(&destination).expect("blocking directory");

        assert!(
            crate::set_instance_icon(&database, &instances, &entry.folder, &picked)
                .await
                .is_err()
        );
        assert!(!destination.with_extension("png.part").exists());
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn download_rejects_disallowed_urls_without_writing() {
        let temp = TempDir::new("icon");
        let dir = temp.path();
        let client = Client::new();
        let result = download_instance_icon(&client, dir, "http://example.com/icon.png").await;
        assert!(result.is_err());
        assert!(!dir.join(ICON_FILE).exists());
    }

    #[tokio::test]
    async fn cached_icons_are_shared_by_url_and_reused_without_network() {
        let temp = TempDir::new("icon-cache");
        let root = temp.path().to_path_buf();
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
        let temp = TempDir::new("icon-duplicate");
        let root = temp.path().to_path_buf();
        let instances = root.join("instances");
        let settings = crate::settings::load_settings_at(&crate::SharedCache::at(&root))
            .await
            .expect("settings");
        let service = crate::DatabaseService::start(
            crate::Database::open(&crate::SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let entry = crate::create_instance(
            &service.client(),
            &instances,
            "Source",
            "1.21",
            &crate::global_launch(&settings),
        )
        .await
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
    }
}
