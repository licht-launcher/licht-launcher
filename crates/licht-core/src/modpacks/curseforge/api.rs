use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::{FILES_URL, MOD_URL};
use crate::modpacks::types::{ModpackListItem, ModpackSource, ModpackVersionItem};
use crate::{CoreError, Result};

/// Ids per `POST /v1/mods/files` and `POST /v1/mods` call.
/// The published schema does not state a maximum. 100 is a batch size other
/// CurseForge clients send successfully.
const FILE_BATCH: usize = 100;

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

#[derive(Debug, Clone)]
pub(super) struct RemoteFile {
    pub(super) file_id: u64,
    pub(super) mod_id: u64,
    pub(super) file_name: String,
    pub(super) display_name: String,
    pub(super) size: u64,
    pub(super) sha1: Option<String>,
    pub(super) md5: Option<String>,
    /// `None` when `downloadUrl` is null or empty: the author disabled distribution.
    pub(super) download_url: Option<String>,
}

#[derive(Debug)]
pub(super) struct RemoteMod {
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) class_id: Option<u64>,
}

pub(super) fn parse_search(body: &str) -> Result<Vec<ModpackListItem>> {
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

pub(super) fn parse_versions(body: &str) -> Result<Vec<ModpackVersionItem>> {
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

/// Looks up the given file ids and the mods that own them, in API batches.
pub(super) async fn resolve_remote(
    client: &Client,
    key: &str,
    file_ids: Vec<u64>,
) -> Result<(Vec<RemoteFile>, Vec<RemoteMod>)> {
    let file_ids = unique_sorted(file_ids);
    if file_ids.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let remote = fetch_remote_files(client, key, &file_ids).await?;
    let mod_ids = unique_sorted(
        remote
            .iter()
            .map(|file| file.mod_id)
            .filter(|id| *id != 0)
            .collect(),
    );
    let mods = if mod_ids.is_empty() {
        Vec::new()
    } else {
        fetch_remote_mods(client, key, &mod_ids).await?
    };
    Ok((remote, mods))
}

async fn fetch_remote_files(client: &Client, key: &str, ids: &[u64]) -> Result<Vec<RemoteFile>> {
    let mut files = Vec::new();
    for chunk in ids.chunks(FILE_BATCH) {
        let body = post_api(
            client,
            key,
            FILES_URL,
            serde_json::json!({ "fileIds": chunk }),
        )
        .await?;
        files.extend(remote_files_from_value(&body)?);
    }
    Ok(files)
}

async fn fetch_remote_mods(client: &Client, key: &str, ids: &[u64]) -> Result<Vec<RemoteMod>> {
    let mut mods = Vec::new();
    for chunk in ids.chunks(FILE_BATCH) {
        let body = post_api(client, key, MOD_URL, serde_json::json!({ "modIds": chunk })).await?;
        mods.extend(remote_mods_from_value(&body)?);
    }
    Ok(mods)
}

async fn post_api(client: &Client, key: &str, url: &str, body: Value) -> Result<Value> {
    let response = client
        .post(url)
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_api_http_{status}"),
        });
    }
    serde_json::from_str(&text).map_err(CoreError::LoaderJson)
}

#[cfg(test)]
pub(super) fn remote_files_from_body(body: &str) -> Result<Vec<RemoteFile>> {
    let value: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    remote_files_from_value(&value)
}

fn remote_files_from_value(value: &Value) -> Result<Vec<RemoteFile>> {
    let mut files = Vec::new();
    for item in data_array(value)? {
        let file_id = item
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(CoreError::ModpackMeta {
                detail: "curseforge_file_id_missing".into(),
            })?;
        let (sha1, md5) = hashes_of(item)?;
        files.push(RemoteFile {
            file_id,
            mod_id: item.get("modId").and_then(Value::as_u64).unwrap_or(0),
            file_name: item
                .get("fileName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            display_name: item
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            size: item.get("fileLength").and_then(Value::as_u64).unwrap_or(0),
            sha1,
            md5,
            download_url: download_url_of(item)?,
        });
    }
    Ok(files)
}

#[cfg(test)]
pub(super) fn remote_mods_from_body(body: &str) -> Result<Vec<RemoteMod>> {
    let value: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    remote_mods_from_value(&value)
}

fn remote_mods_from_value(value: &Value) -> Result<Vec<RemoteMod>> {
    let mut mods = Vec::new();
    for item in data_array(value)? {
        let Some(id) = item.get("id").and_then(Value::as_u64) else {
            continue;
        };
        mods.push(RemoteMod {
            id,
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            class_id: item.get("classId").and_then(Value::as_u64),
        });
    }
    Ok(mods)
}

fn data_array(value: &Value) -> Result<&Vec<Value>> {
    value
        .get("data")
        .and_then(Value::as_array)
        .ok_or(CoreError::ModpackMeta {
            detail: "curseforge_data_missing".into(),
        })
}

/// `downloadUrl: null` is the API signal that third-party distribution is off.
/// A missing field is treated the same way: the CDN path is not invented.
fn download_url_of(item: &Value) -> Result<Option<String>> {
    match item.get("downloadUrl") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(url)) if url.trim().is_empty() => Ok(None),
        Some(Value::String(url)) => Ok(Some(url.clone())),
        Some(_) => Err(CoreError::ModpackMeta {
            detail: "curseforge_download_url_invalid".into(),
        }),
    }
}

fn hashes_of(item: &Value) -> Result<(Option<String>, Option<String>)> {
    let Some(entries) = item.get("hashes").and_then(Value::as_array) else {
        return Ok((None, None));
    };
    let mut sha1 = None;
    let mut md5 = None;
    for entry in entries {
        let Some(value) = entry.get("value").and_then(Value::as_str) else {
            continue;
        };
        match entry.get("algo").and_then(Value::as_u64) {
            Some(1) => sha1 = parse_hash(value, 40)?,
            Some(2) => md5 = parse_hash(value, 32)?,
            _ => {}
        }
    }
    Ok((sha1, md5))
}

fn parse_hash(value: &str, len: usize) -> Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() != len
        || !trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(CoreError::ModpackMeta {
            detail: "curseforge_hash_invalid".into(),
        });
    }
    Ok(Some(trimmed.to_ascii_lowercase()))
}

fn unique_sorted(mut ids: Vec<u64>) -> Vec<u64> {
    ids.sort_unstable();
    ids.dedup();
    ids
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

    #[test]
    fn reads_sha1_and_md5_and_a_null_download_url() {
        let files = remote_files_from_body(
            r#"{
              "data": [{
                "id": 5,
                "modId": 9,
                "fileName": "sodium.jar",
                "displayName": "Sodium 0.5",
                "fileLength": 12,
                "hashes": [
                  {"value": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", "algo": 2},
                  {"value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "algo": 1}
                ],
                "downloadUrl": "https://mediafilez.forgecdn.net/files/1/2/sodium.jar"
              }, {
                "id": 6,
                "modId": 8,
                "fileName": "hidden.jar",
                "displayName": "Hidden",
                "downloadUrl": null
              }]
            }"#,
        )
        .expect("files");
        assert_eq!(
            files[0].sha1.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            files[0].md5.as_deref(),
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        assert!(files[1].download_url.is_none());
    }

    #[test]
    fn rejects_a_hash_that_cannot_be_checked() {
        let error = remote_files_from_body(
            r#"{"data":[{"id":1,"modId":2,"fileName":"a.jar","hashes":[{"value":"abc","algo":1}],"downloadUrl":null}]}"#,
        )
        .unwrap_err();
        match error {
            CoreError::ModpackMeta { detail } => assert_eq!(detail, "curseforge_hash_invalid"),
            other => panic!("expected an invalid hash, got {other}"),
        }
    }

    #[test]
    fn reads_the_class_id_of_each_mod() {
        let mods = remote_mods_from_body(
            r#"{"data":[{"id":1,"name":"Sodium","classId":6},{"id":2,"name":"No class"}]}"#,
        )
        .expect("mods");
        assert_eq!(mods[0].class_id, Some(6));
        assert_eq!(mods[1].class_id, None);
    }

    #[test]
    fn sorts_file_ids() {
        assert_eq!(unique_sorted(vec![3, 1, 3, 2]), vec![1, 2, 3]);
    }
}
