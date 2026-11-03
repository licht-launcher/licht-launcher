use reqwest::Client;

use crate::loaders::http_cache::fetch_cached;
use crate::loaders::types::{LoaderBuild, LoaderChannel};
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
    let builds = list_forge_like_builds(client, metadata_url, game_version).await?;
    let versions = builds.into_iter().map(|build| build.version);
    Ok(match channel {
        LoaderChannel::Stable | LoaderChannel::Latest => versions.take(1).collect(),
        LoaderChannel::Manual => versions.collect(),
    })
}

/// Every build for a game version, newest first. The maven metadata does not
/// say which builds are stable, so all of them count as stable.
pub async fn list_forge_like_builds(
    client: &Client,
    metadata_url: &str,
    game_version: &str,
) -> Result<Vec<LoaderBuild>> {
    let response = fetch_cached(client, metadata_url).await?;
    if !(200..300).contains(&response.status) {
        return Err(CoreError::LoaderMeta {
            detail: format!("forge_meta_http_{}", response.status),
        });
    }
    Ok(parse_forge_builds(
        &response.body,
        game_version,
        metadata_url,
    ))
}

pub fn parse_forge_builds(xml: &str, game_version: &str, metadata_url: &str) -> Vec<LoaderBuild> {
    let neoforge = metadata_url.contains("neoforged");
    let mut builds = Vec::new();
    for line in xml.lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix("<version>")
            .and_then(|value| value.strip_suffix("</version>"))
        else {
            continue;
        };
        let matches_game = if neoforge {
            // NeoForge versions look like 21.1.x matching MC 1.21.1 via major.minor.
            neo_matches_game(inner, game_version)
        } else {
            inner.starts_with(&format!("{game_version}-"))
        };
        if matches_game {
            builds.push(LoaderBuild {
                version: inner.to_string(),
                stable: true,
            });
        }
    }
    builds.reverse();
    builds
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
        let list = parse_forge_builds(xml, "1.20.1", FORGE_METADATA);
        assert_eq!(
            list.into_iter()
                .map(|build| build.version)
                .collect::<Vec<_>>(),
            vec!["1.20.1-47.2.0".to_string()]
        );
    }

    #[test]
    fn forge_builds_are_newest_first_and_all_stable() {
        let xml = r#"
        <metadata><versioning><versions>
          <version>1.20.1-47.1.0</version>
          <version>1.20.1-47.2.0</version>
        </versions></versioning></metadata>
        "#;
        let builds = parse_forge_builds(xml, "1.20.1", FORGE_METADATA);
        assert_eq!(
            builds,
            vec![
                LoaderBuild {
                    version: "1.20.1-47.2.0".into(),
                    stable: true
                },
                LoaderBuild {
                    version: "1.20.1-47.1.0".into(),
                    stable: true
                },
            ]
        );
    }
}
