use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::files::{DISABLED_SUFFIX, content_directory, content_file};
use super::fingerprints::content_fingerprint;
use super::providers::{fetch_fingerprint_metadata, fetch_remote_metadata};
use super::{ContentResolution, InstanceContentEntry, InstanceContentKind};
use crate::store::repositories::content_fingerprints::ContentFingerprint;
use crate::{Result, SharedCache};

pub(super) async fn resolve_content_entry(
    cache: &SharedCache,
    client: &Client,
    root: &Path,
    known: &BTreeMap<&str, &ContentFingerprint>,
    index: usize,
    mut entry: InstanceContentEntry,
    refresh: bool,
) -> Result<ContentResolution> {
    let filename = if entry.enabled {
        entry.name.clone()
    } else {
        format!("{}{}", entry.name, DISABLED_SUFFIX)
    };
    let path = content_file(root, content_directory(entry.kind), &filename)?;
    let relative = Path::new(content_directory(entry.kind)).join(&filename);
    let relative = relative.to_string_lossy().into_owned();
    let fingerprint = content_fingerprint(
        &path,
        relative.clone(),
        known.get(relative.as_str()).copied(),
        refresh,
    )?;
    let key = metadata_cache_key(&fingerprint.sha1);
    let metadata = match read_metadata_cache(cache, &key)? {
        Some(metadata) => metadata,
        None => {
            let remote = fetch_fingerprint_metadata(client, entry.kind, &fingerprint.sha1).await;
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
    Ok(ContentResolution {
        index,
        entry,
        fingerprint,
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ContentMetadata {
    pub(super) title: String,
    #[serde(default)]
    pub(super) version: Option<String>,
    pub(super) icon_url: Option<String>,
}

pub(super) fn fallback_title(name: &str) -> String {
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
        .cache_dir()?
        .join("content-metadata")
        .join(format!("{key}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?).ok())
}

pub(super) fn write_metadata_cache(
    cache: &SharedCache,
    key: &str,
    metadata: &ContentMetadata,
) -> Result<()> {
    let directory = cache.cache_dir()?.join("content-metadata");
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
