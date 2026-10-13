use std::fs;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::loaders::types::LoaderChannel;
use crate::{CoreError, Result, SharedCache};

pub const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
pub const QUILT_META: &str = "https://meta.quiltmc.org/v3";

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: bool,
}

pub async fn list_fabric_like_versions(
    client: &Client,
    meta_base: &str,
    game_version: &str,
    channel: LoaderChannel,
) -> Result<Vec<String>> {
    let url = format!(
        "{meta_base}/versions/loader/{}",
        urlencoding_game(game_version)
    );
    let response = client.get(&url).send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::LoaderMeta {
            detail: format!("loader_list_http_{status}"),
        });
    }
    parse_fabric_loader_list(&body, channel)
}

pub fn parse_fabric_loader_list(body: &str, channel: LoaderChannel) -> Result<Vec<String>> {
    let entries: Vec<LoaderEntry> = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let versions = match channel {
        LoaderChannel::Stable => entries
            .into_iter()
            .filter(|entry| entry.loader.stable)
            .map(|entry| entry.loader.version)
            .collect::<Vec<_>>(),
        LoaderChannel::Latest | LoaderChannel::Manual => entries
            .into_iter()
            .map(|entry| entry.loader.version)
            .collect::<Vec<_>>(),
    };
    Ok(versions)
}

/// Downloads the loader profile JSON into the shared versions cache and returns its id.
pub async fn install_fabric_like(
    client: &Client,
    cache: &SharedCache,
    meta_base: &str,
    id_prefix: &str,
    game_version: &str,
    channel: LoaderChannel,
    loader_version: Option<&str>,
) -> Result<String> {
    let versions = list_fabric_like_versions(client, meta_base, game_version, channel).await?;
    let loader = match loader_version {
        Some(explicit) if !explicit.is_empty() => explicit.to_string(),
        _ => versions
            .into_iter()
            .next()
            .ok_or(CoreError::LoaderVersionMissing)?,
    };
    let url = format!(
        "{meta_base}/versions/loader/{}/{}/profile/json",
        urlencoding_game(game_version),
        urlencoding_game(&loader)
    );
    let response = client.get(&url).send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::LoaderMeta {
            detail: format!("loader_profile_http_{status}"),
        });
    }
    let mut profile: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    fill_missing_maven_hashes(client, &mut profile).await?;
    let body = serde_json::to_string(&profile).map_err(CoreError::LoaderJson)?;
    let id = profile
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{id_prefix}-{loader}-{game_version}"));
    write_version_json(cache, &id, &body)?;
    Ok(id)
}

async fn fill_missing_maven_hashes(client: &Client, profile: &mut Value) -> Result<()> {
    let Some(libraries) = profile.get_mut("libraries").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for library in libraries {
        let Some(object) = library.as_object_mut() else {
            continue;
        };
        let has_hash = object
            .get("sha1")
            .and_then(Value::as_str)
            .is_some_and(|hash| !hash.is_empty());
        if has_hash {
            continue;
        }
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(base_url) = object.get("url").and_then(Value::as_str) else {
            continue;
        };
        let Some(path) = crate::maven_library_path(name) else {
            continue;
        };
        let hash_url = format!("{}/{}.sha1", base_url.trim_end_matches('/'), path);
        let hash = client
            .get(&hash_url)
            .send()
            .await
            .map_err(|error| CoreError::LoaderMeta {
                detail: format!("library_sha1_request: {error}"),
            })?
            .error_for_status()
            .map_err(|error| CoreError::LoaderMeta {
                detail: format!("library_sha1_http: {error}"),
            })?
            .text()
            .await
            .map_err(|error| CoreError::LoaderMeta {
                detail: format!("library_sha1_body: {error}"),
            })?
            .trim()
            .to_string();
        if hash.len() != 40 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
            return Err(CoreError::LoaderMeta {
                detail: format!("invalid library SHA-1 for {name}"),
            });
        }
        object.insert("sha1".to_string(), Value::String(hash));
    }
    Ok(())
}

fn write_version_json(cache: &SharedCache, id: &str, body: &str) -> Result<()> {
    let path = cache.version_json(id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)?;
    Ok(())
}

fn urlencoding_game(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => ch.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_filters_unstable_loaders() {
        let body = r#"[
          {"loader":{"version":"0.16.0","stable":true}},
          {"loader":{"version":"0.16.1","stable":false}}
        ]"#;
        let stable = parse_fabric_loader_list(body, LoaderChannel::Stable).expect("ok");
        assert_eq!(stable, vec!["0.16.0".to_string()]);
        let latest = parse_fabric_loader_list(body, LoaderChannel::Latest).expect("ok");
        assert_eq!(latest.len(), 2);
    }
}
