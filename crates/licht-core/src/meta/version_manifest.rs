use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CoreError, Result, SharedCache};

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

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
        if known.contains(&id) {
            continue;
        }
        let Ok(json_path) = cache.version_json(&id) else {
            continue;
        };
        if !json_path.is_file() {
            continue;
        }
        let installed = cache.version_jar(&id).is_ok_and(|path| path.is_file());
        summaries.push(VersionSummary {
            id: id.clone(),
            version_type: VersionType::OldAlpha,
            installed,
        });
        known.insert(id);
    }
    summaries
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

pub async fn fetch_version_manifest(client: &reqwest::Client) -> Result<VersionManifest> {
    let body = read_text(client, VERSION_MANIFEST_URL).await?;
    parse_version_manifest(&body)
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
        None => Err(CoreError::VersionMissing),
    }
}
