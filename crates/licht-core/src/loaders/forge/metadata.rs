use reqwest::Client;

use crate::loaders::types::LoaderChannel;
use crate::{CoreError, Result};

pub const FORGE_METADATA: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
pub const NEOFORGE_METADATA: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

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
