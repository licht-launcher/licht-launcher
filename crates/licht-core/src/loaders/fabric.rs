use std::cmp::Ordering;
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
    /// Fabric flags stable builds; Quilt's metadata has no such field.
    #[serde(default)]
    stable: Option<bool>,
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
    // Fabric lists loaders newest first and flags the stable ones. Quilt does
    // neither, so for it the order and the stability come from the version.
    let flagged = entries.iter().all(|entry| entry.loader.stable.is_some());
    let mut loaders: Vec<(String, bool)> = entries
        .into_iter()
        .map(|entry| {
            let stable = entry
                .loader
                .stable
                .unwrap_or_else(|| is_final_release(&entry.loader.version));
            (entry.loader.version, stable)
        })
        .collect();
    if !flagged {
        loaders.sort_by(|left, right| compare_versions(&right.0, &left.0));
    }
    let only_stable = matches!(channel, LoaderChannel::Stable);
    Ok(loaders
        .into_iter()
        .filter(|(_, stable)| !only_stable || *stable)
        .map(|(version, _)| version)
        .collect())
}

/// A version without a pre-release suffix (`0.24.0`, not `0.24.0-beta.1`).
fn is_final_release(version: &str) -> bool {
    let version = version.split('+').next().unwrap_or(version);
    !version.contains('-')
}

/// Orders loader versions like semver: numbers first, and a final release is
/// newer than its own pre-releases.
fn compare_versions(left: &str, right: &str) -> Ordering {
    let (left_numbers, left_suffix) = split_version(left);
    let (right_numbers, right_suffix) = split_version(right);
    left_numbers
        .cmp(&right_numbers)
        .then_with(|| match (left_suffix, right_suffix) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => compare_suffixes(left, right),
        })
}

/// `0.20.0-beta.9` becomes `([0, 20, 0], Some("beta.9"))`.
fn split_version(version: &str) -> (Vec<u64>, Option<&str>) {
    let version = version.split('+').next().unwrap_or(version);
    let (numbers, suffix) = match version.split_once('-') {
        Some((numbers, suffix)) => (numbers, Some(suffix)),
        None => (version, None),
    };
    let numbers = numbers
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    (numbers, suffix)
}

/// Compares pre-release suffixes piece by piece: `beta.9` < `beta.10` < `rc.1`.
fn compare_suffixes(left: &str, right: &str) -> Ordering {
    let mut left_parts = left.split('.');
    let mut right_parts = right.split('.');
    loop {
        let ordering = match (left_parts.next(), right_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left), Some(right)) => match (left.parse::<u64>(), right.parse::<u64>()) {
                (Ok(left), Ok(right)) => left.cmp(&right),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => left.cmp(right),
            },
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
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

    /// Quilt omits `stable` and lists loaders out of order (real versions).
    const QUILT_BODY: &str = r#"[
      {"loader":{"version":"0.20.0-beta.9"}},
      {"loader":{"version":"0.20.0-beta.10"}},
      {"loader":{"version":"0.24.0"}},
      {"loader":{"version":"0.23.1"}},
      {"loader":{"version":"0.24.0-rc.1"}},
      {"loader":{"version":"0.20.0-beta.7"}}
    ]"#;

    #[test]
    fn quilt_stable_is_the_final_releases_newest_first() {
        let stable = parse_fabric_loader_list(QUILT_BODY, LoaderChannel::Stable).expect("ok");
        assert_eq!(stable, vec!["0.24.0".to_string(), "0.23.1".to_string()]);
    }

    #[test]
    fn quilt_latest_starts_at_the_newest_build_even_if_it_is_a_pre_release() {
        let latest = parse_fabric_loader_list(QUILT_BODY, LoaderChannel::Latest).expect("ok");
        assert_eq!(
            latest,
            vec![
                "0.24.0",
                "0.24.0-rc.1",
                "0.23.1",
                "0.20.0-beta.10",
                "0.20.0-beta.9",
                "0.20.0-beta.7",
            ]
        );
    }

    #[test]
    fn quilt_manual_lists_every_build_newest_first() {
        let manual = parse_fabric_loader_list(QUILT_BODY, LoaderChannel::Manual).expect("ok");
        assert_eq!(manual.first().map(String::as_str), Some("0.24.0"));
        assert_eq!(manual.len(), 6);
    }

    #[test]
    fn fabric_keeps_the_order_the_api_gives_when_it_flags_stability() {
        let body = r#"[
          {"loader":{"version":"0.20.1","stable":true}},
          {"loader":{"version":"0.20.2-beta.1","stable":false}},
          {"loader":{"version":"0.19.2","stable":true}}
        ]"#;
        let latest = parse_fabric_loader_list(body, LoaderChannel::Latest).expect("ok");
        assert_eq!(latest, vec!["0.20.1", "0.20.2-beta.1", "0.19.2"]);
        let stable = parse_fabric_loader_list(body, LoaderChannel::Stable).expect("ok");
        assert_eq!(stable, vec!["0.20.1", "0.19.2"]);
    }
}
