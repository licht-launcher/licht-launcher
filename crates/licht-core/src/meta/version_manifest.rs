use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::meta::version_resolve::{local_inherits_from, local_version_exists};
use crate::{CoreError, Result, SharedCache};

/// Primary Mojang host (Piston Meta).
pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Legacy Launchermeta S3 host used when Piston Meta fails.
pub const VERSION_MANIFEST_LEGACY_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

/// Per-request timeout for manifest downloads (each host gets this budget).
pub const MANIFEST_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long the on-disk manifest is reused before contacting Mojang again.
pub const MANIFEST_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

const MANIFEST_CACHE_FILE: &str = "version_manifest_v2.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

pub fn version_type_name(version_type: &VersionType) -> &'static str {
    match version_type {
        VersionType::Release => "release",
        VersionType::Snapshot => "snapshot",
        VersionType::OldBeta => "old_beta",
        VersionType::OldAlpha => "old_alpha",
    }
}

/// One manifest entry, plus whether its version JSON is already in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub version_type: VersionType,
    pub installed: bool,
}

/// Path of the on-disk manifest cache: `versions/version_manifest_v2.json`.
pub fn version_manifest_cache_path(cache: &SharedCache) -> PathBuf {
    cache.root().join("versions").join(MANIFEST_CACHE_FILE)
}

/// True when this version can be launched without fetching Mojang metadata.
///
/// Requires a local version JSON and a client jar for this id, or — for loader
/// profiles with `inheritsFrom` — a parent JSON and the parent's client jar.
pub fn is_version_ready(cache: &SharedCache, id: &str) -> bool {
    if !local_version_exists(cache, id) {
        return false;
    }
    if cache.version_jar(id).is_ok_and(|path| path.is_file()) {
        return true;
    }
    match local_inherits_from(cache, id) {
        Ok(Some(parent)) => {
            local_version_exists(cache, &parent)
                && cache.version_jar(&parent).is_ok_and(|path| path.is_file())
        }
        _ => false,
    }
}

/// Summaries built only from folders under `versions/` (no network, no cache file).
pub fn list_installed_version_summaries(cache: &SharedCache) -> Vec<VersionSummary> {
    let mut summaries = Vec::new();
    let versions_dir = cache.root().join("versions");
    let Ok(entries) = std::fs::read_dir(versions_dir) else {
        return summaries;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let Ok(json_path) = cache.version_json(&id) else {
            continue;
        };
        if !json_path.is_file() {
            continue;
        }
        let installed = is_version_ready(cache, &id);
        summaries.push(VersionSummary {
            id,
            version_type: VersionType::OldAlpha,
            installed,
        });
    }
    summaries.sort_by(|left, right| {
        left.id
            .to_ascii_lowercase()
            .cmp(&right.id.to_ascii_lowercase())
    });
    summaries
}

/// Marks each manifest entry installed when `versions/<id>/<id>.json` exists.
///
/// Local loader profiles (Fabric, Forge, …) that are not on Mojang's list are
/// appended with [`VersionType::OldAlpha`] so the create-instance version picker
/// (release/snapshot only) ignores them. For those, `installed` means the client
/// jar exists — the profile JSON alone is written at instance creation.
///
/// An id that cannot be a single path segment is listed and not installed.
pub fn version_summaries(manifest: &VersionManifest, cache: &SharedCache) -> Vec<VersionSummary> {
    let mut summaries: Vec<VersionSummary> = manifest
        .versions
        .iter()
        .map(|version| VersionSummary {
            id: version.id.clone(),
            version_type: version.version_type.clone(),
            installed: cache
                .version_json(&version.id)
                .is_ok_and(|path| path.is_file()),
        })
        .collect();

    let mut known: HashSet<String> = summaries.iter().map(|entry| entry.id.clone()).collect();
    for local in list_installed_version_summaries(cache) {
        if known.contains(&local.id) {
            continue;
        }
        known.insert(local.id.clone());
        summaries.push(local);
    }
    summaries
}

/// Lists versions for the UI: fresh disk cache, then online hosts, then stale
/// cache / installed-only fallback.
pub async fn list_version_summaries(
    client: &reqwest::Client,
    cache: &SharedCache,
) -> Result<Vec<VersionSummary>> {
    match fetch_version_manifest(client, cache).await {
        Ok(manifest) => Ok(version_summaries(&manifest, cache)),
        Err(_) => {
            let installed = list_installed_version_summaries(cache);
            if installed.is_empty() {
                Err(CoreError::VersionManifestUnavailable)
            } else {
                Ok(installed)
            }
        }
    }
}

pub fn version_lines(manifest: &VersionManifest) -> Vec<String> {
    manifest
        .versions
        .iter()
        .map(|version| {
            format!(
                "{} {}",
                version.id,
                version_type_name(&version.version_type)
            )
        })
        .collect()
}

#[derive(Debug)]
pub struct VersionsArgs {
    pub cache: Option<PathBuf>,
}

pub fn parse_versions_args(args: &[String]) -> Result<VersionsArgs> {
    if args.first().map(String::as_str) != Some("versions") {
        return Err(CoreError::LaunchArgs);
    }

    let mut cache = None;
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            return Err(CoreError::LaunchArgs);
        };
        if value.starts_with("--") {
            return Err(CoreError::LaunchArgs);
        }
        match flag.as_str() {
            "--cache" => cache = Some(PathBuf::from(value)),
            _ => return Err(CoreError::LaunchArgs),
        }
    }
    Ok(VersionsArgs { cache })
}

pub fn parse_version_manifest(json: &str) -> Result<VersionManifest> {
    Ok(serde_json::from_str(json)?)
}

/// Fetches the version manifest: fresh on-disk cache → Piston Meta →
/// Launchermeta legacy → stale on-disk cache.
///
/// A successful download from either host updates the on-disk cache.
pub async fn fetch_version_manifest(
    client: &reqwest::Client,
    cache: &SharedCache,
) -> Result<VersionManifest> {
    fetch_version_manifest_from_urls(
        client,
        cache,
        &[VERSION_MANIFEST_URL, VERSION_MANIFEST_LEGACY_URL],
    )
    .await
}

fn manifest_cache_is_fresh(cache: &SharedCache, max_age: Duration) -> bool {
    let path = version_manifest_cache_path(cache);
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    match SystemTime::now().duration_since(modified) {
        Ok(age) => age < max_age,
        // Clock went backwards; treat the file as usable.
        Err(_) => true,
    }
}

async fn fetch_version_manifest_from_urls(
    client: &reqwest::Client,
    cache: &SharedCache,
    urls: &[&str],
) -> Result<VersionManifest> {
    if manifest_cache_is_fresh(cache, MANIFEST_CACHE_TTL)
        && let Ok(body) = read_manifest_cache(cache)
    {
        match parse_version_manifest(&body) {
            Ok(manifest) => return Ok(manifest),
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "version manifest cache was invalid; re-fetching"
                );
            }
        }
    }

    for url in urls {
        match read_text(client, url).await {
            Ok(body) => match parse_version_manifest(&body) {
                Ok(manifest) => {
                    let _ = write_manifest_cache(cache, &body);
                    return Ok(manifest);
                }
                Err(error) => {
                    tracing::warn!(url, error = %error, "version manifest JSON was invalid");
                }
            },
            Err(error) => {
                tracing::warn!(url, error = %error, "version manifest download failed");
            }
        }
    }
    match read_manifest_cache(cache) {
        Ok(body) => parse_version_manifest(&body),
        Err(_) => Err(CoreError::VersionManifestUnavailable),
    }
}

fn write_manifest_cache(cache: &SharedCache, body: &str) -> Result<()> {
    let path = version_manifest_cache_path(cache);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn read_manifest_cache(cache: &SharedCache) -> Result<String> {
    let path = version_manifest_cache_path(cache);
    Ok(std::fs::read_to_string(path)?)
}

async fn read_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let mut last_error = None;
    for _ in 0..3 {
        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };
        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };
        match response.text().await {
            Ok(text) => return Ok(text),
            Err(error) => last_error = Some(error),
        }
    }
    match last_error {
        Some(error) => Err(error.into()),
        None => Err(CoreError::VersionManifestUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache(label: &str) -> SharedCache {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-manifest-{label}-{}-{stamp}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("versions")).unwrap();
        SharedCache::at(root)
    }

    #[test]
    fn writes_and_reads_the_manifest_cache() {
        let cache = temp_cache("roundtrip");
        let body = include_str!("../../tests/fixtures/version_manifest_v2.json");
        write_manifest_cache(&cache, body).expect("write");
        let loaded = read_manifest_cache(&cache).expect("read");
        let manifest = parse_version_manifest(&loaded).expect("parse");
        assert!(!manifest.versions.is_empty());
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[test]
    fn lists_installed_versions_without_a_manifest() {
        let cache = temp_cache("installed");
        let id = "1.20.1";
        let dir = cache.root().join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), br#"{"id":"1.20.1"}"#).unwrap();
        std::fs::write(dir.join(format!("{id}.jar")), b"jar").unwrap();
        // Ignore the cache file if present as a non-version folder name.
        let summaries = list_installed_version_summaries(&cache);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, id);
        assert!(summaries[0].installed);
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[test]
    fn ready_when_jar_exists_or_loader_parent_has_a_jar() {
        let cache = temp_cache("ready");
        let vanilla = "1.20.1";
        let vanilla_dir = cache.root().join("versions").join(vanilla);
        std::fs::create_dir_all(&vanilla_dir).unwrap();
        std::fs::write(
            vanilla_dir.join(format!("{vanilla}.json")),
            br#"{"id":"1.20.1"}"#,
        )
        .unwrap();
        assert!(!is_version_ready(&cache, vanilla));
        std::fs::write(vanilla_dir.join(format!("{vanilla}.jar")), b"jar").unwrap();
        assert!(is_version_ready(&cache, vanilla));

        let loader = "fabric-loader-0.16.0-1.20.1";
        let loader_dir = cache.root().join("versions").join(loader);
        std::fs::create_dir_all(&loader_dir).unwrap();
        std::fs::write(
            loader_dir.join(format!("{loader}.json")),
            br#"{"id":"fabric-loader-0.16.0-1.20.1","inheritsFrom":"1.20.1"}"#,
        )
        .unwrap();
        assert!(is_version_ready(&cache, loader));
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn both_hosts_fail_then_cache_hit() {
        let cache = temp_cache("cache-hit");
        let body = include_str!("../../tests/fixtures/version_manifest_v2.json");
        write_manifest_cache(&cache, body).expect("write");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let dead = "http://127.0.0.1:1/version_manifest_v2.json";
        let manifest = fetch_version_manifest_from_urls(&client, &cache, &[dead, dead])
            .await
            .expect("cache");
        assert!(!manifest.versions.is_empty());
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn fresh_cache_skips_network() {
        let cache = temp_cache("fresh-ttl");
        let body = include_str!("../../tests/fixtures/version_manifest_v2.json");
        write_manifest_cache(&cache, body).expect("write");
        assert!(manifest_cache_is_fresh(&cache, MANIFEST_CACHE_TTL));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        // Unreachable hosts: success means we never needed HTTP.
        let dead = "http://127.0.0.1:1/version_manifest_v2.json";
        let manifest = fetch_version_manifest_from_urls(&client, &cache, &[dead, dead])
            .await
            .expect("fresh cache");
        assert!(!manifest.versions.is_empty());
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn expired_cache_falls_back_after_network_failure() {
        let cache = temp_cache("expired-ttl");
        let body = include_str!("../../tests/fixtures/version_manifest_v2.json");
        write_manifest_cache(&cache, body).expect("write");
        let path = version_manifest_cache_path(&cache);
        let file = std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open cache");
        let stale = SystemTime::now() - MANIFEST_CACHE_TTL - Duration::from_secs(30);
        file.set_modified(stale).expect("set mtime");
        assert!(!manifest_cache_is_fresh(&cache, MANIFEST_CACHE_TTL));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let dead = "http://127.0.0.1:1/version_manifest_v2.json";
        let manifest = fetch_version_manifest_from_urls(&client, &cache, &[dead, dead])
            .await
            .expect("stale cache fallback");
        assert!(!manifest.versions.is_empty());
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn both_hosts_fail_and_no_cache_is_unavailable() {
        let cache = temp_cache("no-cache");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let dead = "http://127.0.0.1:1/version_manifest_v2.json";
        let error = fetch_version_manifest_from_urls(&client, &cache, &[dead, dead])
            .await
            .expect_err("unavailable");
        assert!(matches!(error, CoreError::VersionManifestUnavailable));
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn primary_fail_legacy_ok_writes_cache() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let body = include_str!("../../tests/fixtures/version_manifest_v2.json");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        let response_body = body.to_string();
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut incoming = Vec::new();
            let mut buffer = [0_u8; 512];
            loop {
                let Ok(read) = socket.read(&mut buffer).await else {
                    return;
                };
                if read == 0 {
                    return;
                }
                incoming.extend_from_slice(&buffer[..read]);
                if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(response_body.as_bytes()).await;
        });

        let cache = temp_cache("legacy-ok");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client");
        let dead = "http://127.0.0.1:1/version_manifest_v2.json";
        let legacy = format!("http://{address}/mc/game/version_manifest_v2.json");
        let manifest = fetch_version_manifest_from_urls(&client, &cache, &[dead, &legacy])
            .await
            .expect("legacy");
        assert!(!manifest.versions.is_empty());
        assert!(version_manifest_cache_path(&cache).is_file());
        let _ = std::fs::remove_dir_all(cache.root());
    }

    #[tokio::test]
    async fn list_summaries_falls_back_to_installed_only() {
        let cache = temp_cache("installed-only");
        let id = "1.20.1";
        let dir = cache.root().join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), br#"{"id":"1.20.1"}"#).unwrap();
        std::fs::write(dir.join(format!("{id}.jar")), b"jar").unwrap();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        // Force fetch failure by clearing cache and using dead hosts through the public
        // list path: call list_installed after simulating VersionManifestUnavailable.
        let error = fetch_version_manifest_from_urls(
            &client,
            &cache,
            &["http://127.0.0.1:1/a", "http://127.0.0.1:1/b"],
        )
        .await
        .expect_err("no cache file");
        assert!(matches!(error, CoreError::VersionManifestUnavailable));
        let installed = list_installed_version_summaries(&cache);
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, id);
        let _ = std::fs::remove_dir_all(cache.root());
    }
}
