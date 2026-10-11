use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::modpacks::types::{ModpackListItem, ModpackSource, ModpackVersionItem};
use crate::{CoreError, Result};

const SEARCH_URL: &str = "https://api.curseforge.com/v1/mods/search";
const MOD_URL: &str = "https://api.curseforge.com/v1/mods";
const GAME_ID: &str = "432";
const CLASS_ID: &str = "4471";

pub const CURSEFORGE_API_KEY_ENV: &str = "LICHT_CURSEFORGE_API_KEY";

pub fn curseforge_api_key() -> Result<String> {
    match std::env::var(CURSEFORGE_API_KEY_ENV) {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(CoreError::CurseforgeKeyMissing),
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<ModHit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModHit {
    id: u64,
    name: String,
    summary: String,
    download_count: u64,
    #[serde(default)]
    logo: Option<Logo>,
    #[serde(default)]
    authors: Vec<Author>,
}

#[derive(Debug, Deserialize)]
struct Logo {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: String,
}

pub async fn search_curseforge(client: &Client, query: &str) -> Result<Vec<ModpackListItem>> {
    let key = curseforge_api_key()?;
    let mut url = Url::parse(SEARCH_URL).map_err(|_| CoreError::ModpackMeta {
        detail: "curseforge_search_url".into(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("gameId", GAME_ID);
        pairs.append_pair("classId", CLASS_ID);
        pairs.append_pair("sortField", "2");
        pairs.append_pair("sortOrder", "desc");
        pairs.append_pair("pageSize", "20");
        if !query.trim().is_empty() {
            pairs.append_pair("searchFilter", query.trim());
        }
    }
    let response = client
        .get(url)
        .header("x-api-key", &key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_search_http_{status}"),
        });
    }
    parse_search(&body)
}

pub fn parse_search(body: &str) -> Result<Vec<ModpackListItem>> {
    let parsed: SearchResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    Ok(parsed
        .data
        .into_iter()
        .map(|hit| ModpackListItem {
            id: hit.id.to_string(),
            source: ModpackSource::Curseforge,
            name: hit.name,
            description: hit.summary,
            downloads: hit.download_count,
            icon_url: hit.logo.and_then(|logo| logo.url),
            author: hit
                .authors
                .first()
                .map(|author| author.name.clone())
                .unwrap_or_default(),
        })
        .collect())
}

pub async fn list_curseforge_versions(
    client: &Client,
    mod_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    let key = curseforge_api_key()?;
    let mut url =
        Url::parse(&format!("{MOD_URL}/{mod_id}/files")).map_err(|_| CoreError::ModpackMeta {
            detail: "curseforge_files_url".into(),
        })?;
    url.query_pairs_mut().append_pair("pageSize", "20");
    let response = client
        .get(url)
        .header("x-api-key", &key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_files_http_{status}"),
        });
    }
    parse_versions(&body)
}

pub fn parse_versions(body: &str) -> Result<Vec<ModpackVersionItem>> {
    let parsed: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let files = parsed
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(files
        .into_iter()
        .filter_map(|file| {
            let id = file.get("id")?.as_u64()?.to_string();
            let name = file
                .get("displayName")
                .and_then(Value::as_str)
                .or_else(|| file.get("fileName").and_then(Value::as_str))?
                .to_string();
            let game_versions = file
                .get("gameVersions")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .filter(|item| item.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
                        .collect()
                })
                .unwrap_or_default();
            let loaders = file
                .get("gameVersions")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .filter(|item| {
                            matches!(
                                item.to_ascii_lowercase().as_str(),
                                "fabric" | "forge" | "quilt" | "neoforge"
                            )
                        })
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

pub async fn download_curseforge_pack(
    client: &Client,
    mod_id: &str,
    file_id: &str,
) -> Result<(String, Vec<u8>)> {
    let key = curseforge_api_key()?;
    let url = format!("{MOD_URL}/{mod_id}/files/{file_id}/download-url");
    let response = client
        .get(&url)
        .header("x-api-key", &key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_download_url_http_{status}"),
        });
    }
    let parsed: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    let download_url =
        parsed
            .get("data")
            .and_then(Value::as_str)
            .ok_or(CoreError::ModpackMeta {
                detail: "curseforge_download_url_missing".into(),
            })?;
    let bytes = client
        .get(download_url)
        .header("x-api-key", &key)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();
    Ok((format!("{file_id}.zip"), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_curseforge_search() {
        let body = r#"{
          "data":[{
            "id":1,
            "name":"All the Mods",
            "summary":"big pack",
            "downloadCount":9,
            "logo":{"url":"https://example.com/i.png"},
            "authors":[{"name":"ATM"}]
          }]
        }"#;
        let items = parse_search(body).expect("ok");
        assert_eq!(items[0].id, "1");
        assert_eq!(items[0].source, ModpackSource::Curseforge);
    }
}
