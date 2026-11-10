use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::{CLASS_ID, FILES_URL, MOD_URL};
use crate::modpacks::description::sanitize_html;
use crate::modpacks::types::{
    ModpackDetails, ModpackListItem, ModpackSource, ModpackVersionItem, SourcePage,
};
use crate::{CoreError, Result};

/// Ids per `POST /v1/mods/files` and `POST /v1/mods` call.
/// The published schema does not state a maximum. 100 is a batch size other
/// CurseForge clients send successfully.
const FILE_BATCH: usize = 100;

/// CurseForge rejects searches where `index + pageSize` exceeds this value.
const SEARCH_WINDOW: u64 = 10_000;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<ModHit>,
    #[serde(default)]
    pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagination {
    #[serde(default)]
    total_count: u64,
}

#[derive(Debug, Deserialize)]
struct ModResponse {
    data: ModDetails,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModDetails {
    id: u64,
    name: String,
    summary: String,
    download_count: u64,
    #[serde(default)]
    class_id: Option<u64>,
    #[serde(default)]
    logo: Option<Logo>,
    #[serde(default)]
    authors: Vec<Author>,
    #[serde(default)]
    categories: Vec<Category>,
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    screenshots: Vec<Screenshot>,
    #[serde(default)]
    links: Option<Links>,
}

#[derive(Debug, Deserialize)]
struct Category {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Screenshot {
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DescriptionResponse {
    data: String,
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

pub(super) fn parse_search(body: &str) -> Result<SourcePage> {
    let parsed: SearchResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    // Hits past the search window cannot be paged to, so they do not count.
    let total = parsed
        .pagination
        .map(|pagination| pagination.total_count.min(SEARCH_WINDOW))
        .unwrap_or(0);
    let items = parsed
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
        .collect();
    Ok(SourcePage { items, total })
}

pub(super) fn fits_search_window(index: u64, page_size: u32) -> bool {
    index + u64::from(page_size) <= SEARCH_WINDOW
}

/// A page CurseForge would reject: no items, but the full window still counts,
/// so the merged page count does not shrink.
pub(super) fn past_search_window() -> SourcePage {
    SourcePage {
        items: Vec::new(),
        total: SEARCH_WINDOW,
    }
}

/// Builds the store page from `GET /v1/mods/{id}` and `GET /v1/mods/{id}/description`.
pub(super) fn parse_mod(mod_body: &str, description_body: &str) -> Result<ModpackDetails> {
    let parsed: ModResponse = serde_json::from_str(mod_body).map_err(CoreError::LoaderJson)?;
    let description: DescriptionResponse =
        serde_json::from_str(description_body).map_err(CoreError::LoaderJson)?;
    let project = parsed.data;
    if !project
        .class_id
        .is_some_and(|id| id.to_string() == CLASS_ID)
    {
        return Err(CoreError::ModpackMeta {
            detail: "curseforge_mod_not_modpack".into(),
        });
    }
    Ok(ModpackDetails {
        id: project.id.to_string(),
        source: ModpackSource::Curseforge,
        name: project.name,
        summary: project.summary,
        author: project
            .authors
            .first()
            .map(|author| author.name.clone())
            .unwrap_or_default(),
        downloads: project.download_count,
        icon_url: project.logo.and_then(|logo| logo.url),
        categories: project
            .categories
            .into_iter()
            .map(|category| category.name)
            .collect(),
        updated_at: project.date_modified,
        gallery: project
            .screenshots
            .into_iter()
            .map(|screenshot| screenshot.url)
            .filter(|url| url.starts_with("https://"))
            .collect(),
        project_url: project
            .links
            .and_then(|links| links.website_url)
            .filter(|url| url.starts_with("https://")),
        description_html: sanitize_html(&description.data),
    })
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

pub(super) async fn fetch_remote_files(
    client: &Client,
    key: &str,
    ids: &[u64],
) -> Result<Vec<RemoteFile>> {
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

pub(super) async fn get_api(client: &Client, key: &str, url: &str, label: &str) -> Result<String> {
    let response = client
        .get(url)
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("{label}_http_{status}"),
        });
    }
    Ok(body)
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
        let page = parse_search(body).expect("ok");
        assert_eq!(page.items[0].id, "1");
        assert_eq!(page.items[0].source, ModpackSource::Curseforge);
        assert_eq!(page.total, 0);
    }

    #[test]
    fn reads_the_search_total_count() {
        let page = parse_search(
            r#"{"data":[],"pagination":{"index":0,"pageSize":10,"resultCount":0,"totalCount":5321}}"#,
        )
        .expect("ok");
        assert_eq!(page.total, 5321);
    }

    #[test]
    fn caps_the_total_at_the_search_window() {
        let page = parse_search(r#"{"data":[],"pagination":{"totalCount":250000}}"#).expect("ok");
        assert_eq!(page.total, 10_000);
    }

    #[test]
    fn pages_past_the_search_window_are_not_requested() {
        assert!(fits_search_window(9_990, 10));
        assert!(!fits_search_window(10_000, 10));
        let page = past_search_window();
        assert!(page.items.is_empty());
        assert_eq!(page.total, 10_000);
    }

    #[test]
    fn parses_a_mod_page_with_a_sanitized_description() {
        let project = r#"{
          "data":{
            "id":381671,
            "name":"All the Mods 9",
            "summary":"big pack",
            "downloadCount":99,
            "classId":4471,
            "logo":{"url":"https://media.forgecdn.net/logo.png"},
            "authors":[{"name":"ATMTeam"}],
            "categories":[{"name":"Kitchen Sink"}],
            "dateModified":"2026-10-01T00:00:00Z",
            "screenshots":[{"url":"https://media.forgecdn.net/s.png"}],
            "links":{"websiteUrl":"https://www.curseforge.com/minecraft/modpacks/all-the-mods-9"}
          }
        }"#;
        let description = r#"{"data":"<p>Hello</p><img src=\"x\" onerror=\"alert(1)\">"}"#;
        let details = parse_mod(project, description).expect("ok");
        assert_eq!(details.id, "381671");
        assert_eq!(details.author, "ATMTeam");
        assert_eq!(details.categories, vec!["Kitchen Sink"]);
        assert_eq!(details.gallery, vec!["https://media.forgecdn.net/s.png"]);
        assert_eq!(
            details.project_url.as_deref(),
            Some("https://www.curseforge.com/minecraft/modpacks/all-the-mods-9")
        );
        assert!(details.description_html.contains("<p>Hello</p>"));
        assert!(!details.description_html.contains("onerror"));
    }

    #[test]
    fn rejects_a_mod_that_is_not_a_modpack() {
        let project =
            r#"{"data":{"id":1,"name":"JEI","summary":"","downloadCount":1,"classId":6}}"#;
        assert!(parse_mod(project, r#"{"data":""}"#).is_err());
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
