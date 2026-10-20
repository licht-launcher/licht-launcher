use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cache::SharedCache;
use crate::hashes::sha1_hex;
use crate::{CoreError, Result};

const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum InstanceContentKind {
    Mods,
    ResourcePacks,
    ShaderPacks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceContentEntry {
    pub name: String,
    pub kind: InstanceContentKind,
    pub enabled: bool,
    #[ts(type = "number")]
    pub size: u64,
    pub title: String,
    pub version: Option<String>,
    pub icon_url: Option<String>,
}

pub fn list_instance_content(
    cache: &SharedCache,
    folder: &str,
) -> Result<Vec<InstanceContentEntry>> {
    let root = instance_root(cache, folder)?;
    let mut entries = Vec::new();
    for (directory, kind) in [
        ("mods", InstanceContentKind::Mods),
        ("resourcepacks", InstanceContentKind::ResourcePacks),
        ("shaderpacks", InstanceContentKind::ShaderPacks),
    ] {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        for item in fs::read_dir(path)? {
            let item = item?;
            let metadata = item.metadata()?;
            if !metadata.is_file() {
                continue;
            }
            let raw_name = item.file_name().to_string_lossy().into_owned();
            let enabled = !raw_name.ends_with(DISABLED_SUFFIX);
            let name = raw_name
                .strip_suffix(DISABLED_SUFFIX)
                .unwrap_or(&raw_name)
                .to_string();
            entries.push(InstanceContentEntry {
                title: fallback_title(&name),
                name,
                kind,
                enabled,
                size: metadata.len(),
                version: None,
                icon_url: None,
            });
        }
    }
    entries.sort_by(|left, right| {
        left.kind
            .sort_key()
            .cmp(&right.kind.sort_key())
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
    });
    Ok(entries)
}

pub async fn list_instance_content_with_metadata(
    cache: &SharedCache,
    client: &Client,
    folder: &str,
    refresh: bool,
) -> Result<Vec<InstanceContentEntry>> {
    let root = instance_root(cache, folder)?;
    let mut entries = list_instance_content(cache, folder)?;
    for entry in &mut entries {
        let filename = if entry.enabled {
            entry.name.clone()
        } else {
            format!("{}{}", entry.name, DISABLED_SUFFIX)
        };
        let path = content_file(&root, content_directory(entry.kind), &filename)?;
        let fingerprint = sha1_hex(&fs::read(&path)?);
        let key = metadata_cache_key(&fingerprint);
        let metadata = match (!refresh)
            .then(|| read_metadata_cache(cache, &key))
            .transpose()?
        {
            Some(Some(metadata)) => metadata,
            Some(None) | None => {
                let remote = fetch_fingerprint_metadata(client, entry.kind, &fingerprint).await;
                let local = if remote.is_none() {
                    read_local_metadata(&path, entry.kind)
                } else {
                    None
                };
                let fallback = if remote.is_none() && local.is_none() {
                    fetch_remote_metadata(client, entry.kind, &entry.name).await
                } else {
                    None
                };
                let metadata = remote
                    .or(local)
                    .or(fallback)
                    .unwrap_or_else(|| ContentMetadata {
                        title: fallback_title(&entry.name),
                        version: None,
                        icon_url: None,
                    });
                write_metadata_cache(cache, &key, &metadata)?;
                metadata
            }
        };
        entry.title = metadata.title;
        entry.version = metadata.version;
        entry.icon_url = metadata.icon_url;
    }
    Ok(entries)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ContentMetadata {
    title: String,
    #[serde(default)]
    version: Option<String>,
    icon_url: Option<String>,
}

fn fallback_title(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(title, _)| title)
        .unwrap_or(name)
        .replace(['-', '_', '.'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn metadata_cache_key(fingerprint: &str) -> String {
    fingerprint.to_ascii_lowercase()
}

fn read_metadata_cache(cache: &SharedCache, key: &str) -> Result<Option<ContentMetadata>> {
    let path = cache
        .root()
        .join("content-metadata")
        .join(format!("{key}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?).ok())
}

fn write_metadata_cache(cache: &SharedCache, key: &str, metadata: &ContentMetadata) -> Result<()> {
    let directory = cache.root().join("content-metadata");
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join(format!("{key}.json")),
        serde_json::to_vec(metadata)?,
    )?;
    Ok(())
}

fn read_local_metadata(path: &Path, kind: InstanceContentKind) -> Option<ContentMetadata> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let candidates: &[&str] = match kind {
        InstanceContentKind::Mods => &["fabric.mod.json", "quilt.mod.json", "META-INF/mods.toml"],
        InstanceContentKind::ResourcePacks | InstanceContentKind::ShaderPacks => &["pack.mcmeta"],
    };
    for candidate in candidates {
        let Ok(mut entry) = archive.by_name(candidate) else {
            continue;
        };
        let mut body = String::new();
        entry.read_to_string(&mut body).ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;
        let title = json
            .get("name")
            .and_then(serde_json::Value::as_str)
            .or_else(|| json.get("pack")?.get("description")?.as_str())?;
        return Some(ContentMetadata {
            title: title.to_string(),
            version: json
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            icon_url: None,
        });
    }
    None
}

async fn fetch_remote_metadata(
    client: &Client,
    kind: InstanceContentKind,
    name: &str,
) -> Option<ContentMetadata> {
    let query = fallback_title(name);
    first_metadata(
        fetch_modrinth_metadata(client, kind, &query),
        fetch_curseforge_metadata(client, kind, &query),
    )
    .await
}

async fn fetch_fingerprint_metadata(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    first_metadata(
        fetch_modrinth_fingerprint(client, kind, fingerprint),
        fetch_curseforge_fingerprint(client, kind, fingerprint),
    )
    .await
}

async fn first_metadata(
    primary: impl std::future::Future<Output = Option<ContentMetadata>>,
    fallback: impl std::future::Future<Output = Option<ContentMetadata>>,
) -> Option<ContentMetadata> {
    match primary.await {
        Some(metadata) => Some(metadata),
        None => fallback.await,
    }
}

async fn fetch_modrinth_fingerprint(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    #[derive(Deserialize)]
    struct Version {
        project_id: String,
        version_number: String,
    }
    #[derive(Deserialize)]
    struct Project {
        title: String,
        icon_url: Option<String>,
        project_type: String,
    }
    let version = client
        .get(format!(
            "https://api.modrinth.com/v2/version_file/{fingerprint}?algorithm=sha1"
        ))
        .send()
        .await
        .ok()?
        .json::<Version>()
        .await
        .ok()?;
    let project = client
        .get(format!(
            "https://api.modrinth.com/v2/project/{}",
            version.project_id
        ))
        .send()
        .await
        .ok()?
        .json::<Project>()
        .await
        .ok()?;
    let expected = match kind {
        InstanceContentKind::Mods => "mod",
        InstanceContentKind::ResourcePacks => "resourcepack",
        InstanceContentKind::ShaderPacks => "shader",
    };
    (project.project_type == expected).then_some(ContentMetadata {
        title: project.title,
        version: Some(version.version_number),
        icon_url: project.icon_url,
    })
}

async fn fetch_curseforge_fingerprint(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    let key = std::env::var("LICHT_CURSEFORGE_API_KEY").ok()?;
    #[derive(Deserialize)]
    struct FingerprintResponse {
        data: FingerprintData,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintData {
        exact_matches: Vec<FingerprintMatch>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintMatch {
        file: FingerprintFile,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintFile {
        mod_id: u64,
        display_name: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModResponse {
        data: ModData,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModData {
        class_id: Option<u64>,
        name: String,
        logo: Option<Logo>,
    }
    #[derive(Deserialize)]
    struct Logo {
        url: Option<String>,
    }
    let response = client
        .get(format!(
            "https://api.curseforge.com/v1/fingerprints/{fingerprint}"
        ))
        .header("x-api-key", key.clone())
        .send()
        .await
        .ok()?
        .json::<FingerprintResponse>()
        .await
        .ok()?;
    let mod_id = response.data.exact_matches.first()?.file.mod_id;
    let project = client
        .get(format!("https://api.curseforge.com/v1/mods/{mod_id}"))
        .header("x-api-key", key)
        .send()
        .await
        .ok()?
        .json::<ModResponse>()
        .await
        .ok()?;
    let expected = match kind {
        InstanceContentKind::Mods => 6,
        InstanceContentKind::ResourcePacks => 12,
        InstanceContentKind::ShaderPacks => 6552,
    };
    (project.data.class_id == Some(expected)).then_some(ContentMetadata {
        title: project.data.name,
        version: response
            .data
            .exact_matches
            .first()
            .and_then(|item| item.file.display_name.clone()),
        icon_url: project.data.logo.and_then(|logo| logo.url),
    })
}

async fn fetch_modrinth_metadata(
    client: &Client,
    kind: InstanceContentKind,
    query: &str,
) -> Option<ContentMetadata> {
    #[derive(Deserialize)]
    struct Hit {
        title: String,
        icon_url: Option<String>,
        project_type: String,
    }
    #[derive(Deserialize)]
    struct Response {
        hits: Vec<Hit>,
    }
    let expected = match kind {
        InstanceContentKind::Mods => "mod",
        InstanceContentKind::ResourcePacks => "resourcepack",
        InstanceContentKind::ShaderPacks => "shader",
    };
    let url = format!(
        "https://api.modrinth.com/v2/search?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("query", query)
            .append_pair("limit", "10")
            .finish()
    );
    let response = client
        .get(url)
        .send()
        .await
        .ok()?
        .json::<Response>()
        .await
        .ok()?;
    response
        .hits
        .into_iter()
        .find(|hit| hit.project_type == expected)
        .map(|hit| ContentMetadata {
            title: hit.title,
            version: None,
            icon_url: hit.icon_url,
        })
}

async fn fetch_curseforge_metadata(
    client: &Client,
    kind: InstanceContentKind,
    query: &str,
) -> Option<ContentMetadata> {
    let key = std::env::var("LICHT_CURSEFORGE_API_KEY").ok()?;
    let class_id = match kind {
        InstanceContentKind::Mods => 6,
        InstanceContentKind::ResourcePacks => 12,
        InstanceContentKind::ShaderPacks => 6552,
    };
    #[derive(Deserialize)]
    struct Hit {
        name: String,
        logo: Option<Logo>,
    }
    #[derive(Deserialize)]
    struct Logo {
        url: Option<String>,
    }
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Hit>,
    }
    let url = format!(
        "https://api.curseforge.com/v1/mods/search?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("gameId", "432")
            .append_pair("classId", &class_id.to_string())
            .append_pair("searchFilter", query)
            .append_pair("pageSize", "10")
            .finish()
    );
    let response = client
        .get(url)
        .header("x-api-key", key)
        .send()
        .await
        .ok()?
        .json::<Response>()
        .await
        .ok()?;
    response.data.into_iter().next().map(|hit| ContentMetadata {
        title: hit.name,
        version: None,
        icon_url: hit.logo.and_then(|logo| logo.url),
    })
}

pub fn set_instance_content_enabled(
    cache: &SharedCache,
    folder: &str,
    kind: InstanceContentKind,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let root = instance_root(cache, folder)?;
    let current_name = if enabled {
        format!("{name}{DISABLED_SUFFIX}")
    } else {
        name.to_string()
    };
    let next_name = if enabled {
        name.to_string()
    } else {
        format!("{name}{DISABLED_SUFFIX}")
    };
    let current = content_file(&root, content_directory(kind), &current_name)?;
    let next = content_file(&root, content_directory(kind), &next_name)?;
    if !current.is_file() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "content file is missing",
        )));
    }
    if next.exists() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "content file already exists",
        )));
    }
    fs::rename(current, next)?;
    Ok(())
}

pub fn remove_instance_content(
    cache: &SharedCache,
    folder: &str,
    kind: InstanceContentKind,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let root = instance_root(cache, folder)?;
    let filename = if enabled {
        name.to_string()
    } else {
        format!("{name}{DISABLED_SUFFIX}")
    };
    let path = content_file(&root, content_directory(kind), &filename)?;
    if !path.is_file() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "content file is missing",
        )));
    }
    fs::remove_file(path)?;
    Ok(())
}

fn instance_root(cache: &SharedCache, folder: &str) -> Result<PathBuf> {
    let root = cache.instance_dir(folder)?;
    if !root.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    Ok(root)
}

fn content_file(root: &Path, directory: &str, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
        || name.contains(':')
    {
        return Err(CoreError::CachePath);
    }
    Ok(root.join(directory).join(name))
}

fn content_directory(kind: InstanceContentKind) -> &'static str {
    match kind {
        InstanceContentKind::Mods => "mods",
        InstanceContentKind::ResourcePacks => "resourcepacks",
        InstanceContentKind::ShaderPacks => "shaderpacks",
    }
}

impl InstanceContentKind {
    fn sort_key(self) -> u8 {
        match self {
            Self::Mods => 0,
            Self::ResourcePacks => 1,
            Self::ShaderPacks => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn metadata(title: &str) -> ContentMetadata {
        ContentMetadata {
            title: title.to_string(),
            version: Some("1.0".to_string()),
            icon_url: None,
        }
    }

    #[tokio::test]
    async fn metadata_hit_does_not_execute_the_fallback() {
        let primary = async { Some(metadata("Primary")) };
        let fallback = async {
            panic!("a metadata hit must not execute the fallback provider");
        };

        let result = first_metadata(primary, fallback).await.expect("metadata");

        assert_eq!(result.title, "Primary");
        assert_eq!(result.version.as_deref(), Some("1.0"));
    }

    #[tokio::test]
    async fn metadata_miss_awaits_the_primary_before_trying_the_fallback() {
        let primary_finished = std::cell::Cell::new(false);
        let primary = async {
            tokio::task::yield_now().await;
            primary_finished.set(true);
            None
        };
        let fallback = async {
            assert!(primary_finished.get(), "primary lookup must finish first");
            Some(metadata("Fallback"))
        };

        let result = first_metadata(primary, fallback).await.expect("metadata");

        assert_eq!(result.title, "Fallback");
        assert_eq!(result.version.as_deref(), Some("1.0"));
    }

    #[tokio::test]
    async fn metadata_miss_in_both_providers_preserves_the_missing_result() {
        let result = first_metadata(async { None }, async { None }).await;

        assert!(result.is_none());
    }

    fn cache(name: &str) -> SharedCache {
        let root = std::env::temp_dir().join(format!(
            "licht-content-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("data")).expect("cache root");
        SharedCache::at(root.join("data"))
    }

    fn instance(cache: &SharedCache) -> PathBuf {
        let root = cache.instance_dir("classic").expect("instance");
        for directory in ["mods", "resourcepacks", "shaderpacks"] {
            fs::create_dir_all(root.join(directory)).expect("content directory");
        }
        root
    }

    #[test]
    fn lists_and_toggles_content() {
        let cache = cache("list");
        let root = instance(&cache);
        fs::write(root.join("mods").join("sodium.jar"), b"mod").expect("mod");
        fs::write(
            root.join("resourcepacks").join("faithful.zip.disabled"),
            b"pack",
        )
        .expect("pack");

        let entries = list_instance_content(&cache, "classic").expect("list");
        assert_eq!(entries[0].title, "Sodium");
        assert!(!entries[1].enabled);
        set_instance_content_enabled(
            &cache,
            "classic",
            InstanceContentKind::Mods,
            "sodium.jar",
            false,
        )
        .expect("disable");
        assert!(root.join("mods").join("sodium.jar.disabled").is_file());
    }

    #[test]
    fn rejects_unsafe_names_and_conflicts() {
        let cache = cache("validation");
        let root = instance(&cache);
        fs::write(root.join("mods").join("test.jar"), b"mod").expect("mod");
        fs::write(root.join("mods").join("test.jar.disabled"), b"mod").expect("disabled");
        assert!(matches!(
            remove_instance_content(
                &cache,
                "classic",
                InstanceContentKind::Mods,
                "../test.jar",
                true
            ),
            Err(CoreError::CachePath)
        ));
        assert!(matches!(
            set_instance_content_enabled(
                &cache,
                "classic",
                InstanceContentKind::Mods,
                "test.jar",
                false
            ),
            Err(CoreError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
    }
}
