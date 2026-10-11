use std::fs;

use reqwest::Client;
use serde_json::{Value, json};

use crate::loaders::types::LoaderChannel;
use crate::{CoreError, Result, SharedCache};

pub const FORGE_METADATA: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
pub const NEOFORGE_METADATA: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

#[derive(Debug, Clone, Copy)]
pub enum ForgeFamily {
    Forge,
    NeoForge,
}

pub async fn list_forge_like_versions(
    client: &Client,
    metadata_url: &str,
    game_version: &str,
    channel: LoaderChannel,
) -> Result<Vec<String>> {
    let response = client.get(metadata_url).send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::LoaderMeta {
            detail: format!("forge_meta_http_{status}"),
        });
    }
    Ok(parse_forge_versions(
        &body,
        game_version,
        channel,
        metadata_url,
    ))
}

pub fn parse_forge_versions(
    xml: &str,
    game_version: &str,
    channel: LoaderChannel,
    metadata_url: &str,
) -> Vec<String> {
    let neoforge = metadata_url.contains("neoforged");
    let mut versions = Vec::new();
    for line in xml.lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix("<version>")
            .and_then(|value| value.strip_suffix("</version>"))
        else {
            continue;
        };
        if neoforge {
            // NeoForge versions look like 21.1.x matching MC 1.21.1 via major.minor.
            if neo_matches_game(inner, game_version) {
                versions.push(inner.to_string());
            }
        } else if inner.starts_with(&format!("{game_version}-")) {
            versions.push(inner.to_string());
        }
    }
    versions.reverse();
    match channel {
        LoaderChannel::Stable | LoaderChannel::Latest => versions.into_iter().take(1).collect(),
        LoaderChannel::Manual => versions,
    }
}

fn neo_matches_game(neo_version: &str, game_version: &str) -> bool {
    // 1.21.1 -> 21.1 ; 1.20.1 -> 20.1
    let stripped = game_version.strip_prefix("1.").unwrap_or(game_version);
    neo_version.starts_with(&format!("{stripped}."))
        || neo_version.starts_with(&format!("{stripped}-"))
}

/// Writes a thin wrapper version JSON that inherits the vanilla game version.
/// Full Forge/NeoForge processor install runs on first play via the official installer later;
/// this records the chosen loader build so the instance is created consistently.
pub async fn install_forge_like(
    client: &Client,
    cache: &SharedCache,
    family: ForgeFamily,
    game_version: &str,
    channel: LoaderChannel,
    loader_version: Option<&str>,
) -> Result<String> {
    let metadata = match family {
        ForgeFamily::Forge => FORGE_METADATA,
        ForgeFamily::NeoForge => NEOFORGE_METADATA,
    };
    let versions = list_forge_like_versions(client, metadata, game_version, channel).await?;
    let build = match loader_version {
        Some(explicit) if !explicit.is_empty() => explicit.to_string(),
        _ => versions
            .into_iter()
            .next()
            .ok_or(CoreError::LoaderVersionMissing)?,
    };
    let prefix = match family {
        ForgeFamily::Forge => "forge",
        ForgeFamily::NeoForge => "neoforge",
    };
    let id = format!("{prefix}-{build}");
    let profile = json!({
        "id": id,
        "inheritsFrom": game_version,
        "time": "1970-01-01T00:00:00Z",
        "releaseTime": "1970-01-01T00:00:00Z",
        "type": "release",
        "mainClass": "net.minecraft.client.main.Main",
        "arguments": {
            "game": [],
            "jvm": []
        },
        "libraries": [],
        "lichtLoader": {
            "family": prefix,
            "build": build,
            "gameVersion": game_version
        }
    });
    write_version_json(cache, &id, &profile)?;
    let _ = client;
    Ok(id)
}

fn write_version_json(cache: &SharedCache, id: &str, profile: &Value) -> Result<()> {
    let path = cache.version_json(id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(profile).map_err(CoreError::LoaderJson)?;
    fs::write(path, body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_versions_filter_by_game() {
        let xml = r#"
        <metadata>
          <versioning>
            <versions>
              <version>1.20.1-47.2.0</version>
              <version>1.21.1-52.0.0</version>
            </versions>
          </versioning>
        </metadata>
        "#;
        let list = parse_forge_versions(xml, "1.20.1", LoaderChannel::Manual, FORGE_METADATA);
        assert_eq!(list, vec!["1.20.1-47.2.0".to_string()]);
    }
}
