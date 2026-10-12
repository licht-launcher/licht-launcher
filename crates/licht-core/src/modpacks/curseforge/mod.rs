//! CurseForge modpack search, pack download and install.

mod api;
mod cdn;
mod install;

use reqwest::Client;
use serde_json::Value;
use url::Url;

use crate::modpacks::types::{ModpackListItem, ModpackVersionItem};
use crate::{CoreError, Result};

pub use install::{import_curseforge_pack, install_curseforge_files};

const SEARCH_URL: &str = "https://api.curseforge.com/v1/mods/search";
const MOD_URL: &str = "https://api.curseforge.com/v1/mods";
const FILES_URL: &str = "https://api.curseforge.com/v1/mods/files";
const GAME_ID: &str = "432";
const CLASS_ID: &str = "4471";

pub const CURSEFORGE_API_KEY_ENV: &str = "LICHT_CURSEFORGE_API_KEY";

/// Resolved CurseForge files for one instance, parallel to `licht-modpack.json`.
pub const CURSEFORGE_INDEX_FILE: &str = "licht-curseforge.json";

pub fn curseforge_api_key() -> Result<String> {
    match std::env::var(CURSEFORGE_API_KEY_ENV) {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(CoreError::CurseforgeKeyMissing),
    }
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
    api::parse_search(&body)
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
    api::parse_versions(&body)
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
        return Err(download_url_status_error(status, &body));
    }
    let download_url = parse_pack_download_url(&body)?;
    let bytes = cdn::fetch_cdn_bytes(&key, &download_url).await?;
    Ok((format!("{file_id}.zip"), bytes))
}

fn download_url_status_error(status: u16, body: &str) -> CoreError {
    if status == 403 && body.to_ascii_lowercase().contains("distribution") {
        return CoreError::CurseforgePackDistributionDenied;
    }
    CoreError::ModpackMeta {
        detail: format!("curseforge_download_url_http_{status}"),
    }
}

fn parse_pack_download_url(body: &str) -> Result<String> {
    let parsed: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    match parsed.get("data") {
        Some(Value::Null) => Err(CoreError::CurseforgePackDistributionDenied),
        Some(Value::String(url)) => cdn::canonical_download_url(url),
        _ => Err(CoreError::ModpackMeta {
            detail: "curseforge_download_url_missing".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_download_url_must_be_a_cdn_host() {
        let url = parse_pack_download_url(
            r#"{"data":"https://edge.forgecdn.net/files/1/2/pack.zip?api-key=secret"}"#,
        )
        .expect("url");
        assert!(!url.contains("secret"));
        assert!(url.starts_with("https://edge.forgecdn.net/"));
        let denied = parse_pack_download_url(r#"{"data":null}"#).unwrap_err();
        assert!(matches!(
            denied,
            CoreError::CurseforgePackDistributionDenied
        ));
        let github =
            parse_pack_download_url(r#"{"data":"https://github.com/a/pack.zip"}"#).unwrap_err();
        assert!(matches!(github, CoreError::CurseforgeDownloadHost));
    }

    #[test]
    fn a_403_that_mentions_distribution_is_the_pack_error() {
        let error = download_url_status_error(403, "Project distribution is disabled");
        assert!(matches!(error, CoreError::CurseforgePackDistributionDenied));
        let other = download_url_status_error(403, "bad key");
        match other {
            CoreError::ModpackMeta { detail } => {
                assert_eq!(detail, "curseforge_download_url_http_403");
            }
            other => panic!("expected metadata failure, got {other}"),
        }
    }
}
