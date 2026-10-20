use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::modpacks::types::{ModpackListItem, ModpackSource, ModpackVersionItem};
use crate::{CoreError, Result};

const SEARCH_URL: &str = "https://api.modrinth.com/v2/search";
const PROJECT_VERSIONS: &str = "https://api.modrinth.com/v2/project";
const USER_AGENT: &str = "licht-launcher/0.1.1 (https://github.com/licht-launcher/licht-launcher)";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    project_id: String,
    title: String,
    description: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    author: String,
}

pub async fn search_modrinth(client: &Client, query: &str) -> Result<Vec<ModpackListItem>> {
    let mut url = Url::parse(SEARCH_URL).map_err(|_| CoreError::ModpackMeta {
        detail: "modrinth_search_url".into(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("limit", "20");
        pairs.append_pair("index", "downloads");
        pairs.append_pair("facets", r#"[["project_type:modpack"]]"#);
        if !query.trim().is_empty() {
            pairs.append_pair("query", query.trim());
        }
    }
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("modrinth_search_http_{status}"),
        });
    }
    parse_search(&body)
}

pub fn parse_search(body: &str) -> Result<Vec<ModpackListItem>> {
    let parsed: SearchResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    Ok(parsed
        .hits
        .into_iter()
        .map(|hit| ModpackListItem {
            id: hit.project_id,
            source: ModpackSource::Modrinth,
            name: hit.title,
            description: hit.description,
            downloads: hit.downloads,
            icon_url: hit.icon_url,
            author: hit.author,
        })
        .collect())
}

pub async fn list_modrinth_versions(
    client: &Client,
    project_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    let url = format!("{PROJECT_VERSIONS}/{project_id}/version");
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("modrinth_versions_http_{status}"),
        });
    }
    parse_versions(&body)
}

pub fn parse_versions(body: &str) -> Result<Vec<ModpackVersionItem>> {
    let parsed: Vec<Value> = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    Ok(parsed
        .into_iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let game_versions = entry
                .get("game_versions")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let loaders = entry
                .get("loaders")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(ModpackVersionItem {
                id,
                name,
                game_versions,
                loaders,
            })
        })
        .collect())
}

pub(super) async fn download_modrinth_mrpack_to(
    client: &Client,
    project_id: &str,
    version_id: &str,
    destination: &std::path::Path,
    context: &crate::InstallationContext,
) -> Result<()> {
    let versions = context
        .metadata(list_modrinth_versions(client, project_id))
        .await?;
    if !versions.iter().any(|version| version.id == version_id) {
        return Err(CoreError::ModpackMeta {
            detail: "version_not_in_project".into(),
        });
    }
    let url = format!("{PROJECT_VERSIONS}/{project_id}/version/{version_id}");
    let parsed = context
        .metadata(async {
            let response = client
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await?;
            let status = response.status().as_u16();
            let body = response.text().await?;
            if !(200..300).contains(&status) {
                return Err(CoreError::ModpackMeta {
                    detail: format!("modrinth_version_http_{status}"),
                });
            }
            serde_json::from_str::<Value>(&body).map_err(CoreError::LoaderJson)
        })
        .await?;
    let file = parsed
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files
                .iter()
                .find(|file| file.get("primary").and_then(Value::as_bool) == Some(true))
                .or_else(|| files.first())
        })
        .ok_or(CoreError::ModpackMeta {
            detail: "mrpack_file_missing".into(),
        })?;
    let download_url = file
        .get("url")
        .and_then(Value::as_str)
        .ok_or(CoreError::ModpackMeta {
            detail: "mrpack_url_missing".into(),
        })?;
    if !super::secure_download::download_url_allowed(download_url, &["cdn.modrinth.com"]) {
        return Err(CoreError::ModpackMeta {
            detail: "modpack download host is not allowed".into(),
        });
    }
    let checksum = if let Some(value) = file.pointer("/hashes/sha1").and_then(Value::as_str) {
        crate::DownloadChecksum::Sha1(value.to_string())
    } else if let Some(value) = file.pointer("/hashes/sha512").and_then(Value::as_str) {
        crate::DownloadChecksum::Sha512(value.to_string())
    } else {
        return Err(CoreError::ModpackMeta {
            detail: "modpack archive checksum is missing".into(),
        });
    };
    let task = crate::VerifiedDownloadTask {
        url: download_url.to_string(),
        destination: destination.to_path_buf(),
        checksum,
        size: file.get("size").and_then(Value::as_u64).unwrap_or(0),
    };
    context
        .verified_queue(
            &super::mrpack::mrpack_download_client()?,
            &[task],
            context.options()?,
        )
        .await?;
    context.stage(crate::DownloadPhase::Verifying).await?;
    if let Some(expected) = file.pointer("/hashes/sha512").and_then(Value::as_str) {
        super::file_hashes::ensure_sha512(destination, expected, &context.control).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modrinth_search_hits() {
        let body = r#"{
          "hits":[{
            "project_id":"ABC",
            "title":"Cobblemon",
            "description":"Pokemon",
            "downloads":10,
            "icon_url":null,
            "author":"team"
          }]
        }"#;
        let items = parse_search(body).expect("ok");
        assert_eq!(items[0].name, "Cobblemon");
        assert_eq!(items[0].source, ModpackSource::Modrinth);
    }
}
