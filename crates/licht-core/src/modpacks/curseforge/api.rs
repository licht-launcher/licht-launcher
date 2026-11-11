use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::{CLASS_ID, FILES_URL, MOD_URL};
use crate::modpacks::description::sanitize_html;
use crate::modpacks::types::{
    ModpackCategory, ModpackDetails, ModpackImage, ModpackLinks, ModpackListItem, ModpackLoader,
    ModpackSort, ModpackSource, ModpackVersionItem, ModpackVersionType, SourcePage, https_only,
    push_unique,
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
struct CategoriesResponse {
    data: Vec<CategoryEntry>,
}

#[derive(Debug, Deserialize)]
struct CategoryEntry {
    id: u64,
    slug: String,
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
    date_created: Option<String>,
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    screenshots: Vec<Screenshot>,
    #[serde(default)]
    links: Option<Links>,
    #[serde(default)]
    latest_files_indexes: Vec<FileIndex>,
}

#[derive(Debug, Deserialize)]
struct Category {
    slug: String,
}

#[derive(Debug, Deserialize)]
struct Screenshot {
    url: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    #[serde(default)]
    website_url: Option<String>,
    #[serde(default)]
    wiki_url: Option<String>,
    #[serde(default)]
    issues_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    data: Vec<FileEntry>,
    #[serde(default)]
    pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    release_type: Option<u64>,
    #[serde(default)]
    file_date: Option<String>,
    #[serde(default)]
    download_count: u64,
    /// Game versions and loader names together, e.g. `["1.21.1", "NeoForge"]`.
    #[serde(default)]
    game_versions: Vec<String>,
}

/// One page of `GET /v1/mods/{id}/files`, with the total file count.
pub(super) struct FilesPage {
    pub(super) items: Vec<ModpackVersionItem>,
    pub(super) total: u64,
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
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    categories: Vec<HitCategory>,
    #[serde(default)]
    latest_files_indexes: Vec<FileIndex>,
}

#[derive(Debug, Deserialize)]
struct HitCategory {
    slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileIndex {
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    mod_loader: Option<u64>,
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
        .map(|hit| {
            let mut loaders = Vec::new();
            for index in &hit.latest_files_indexes {
                if let Some(loader) = index.mod_loader.and_then(loader_from_type) {
                    push_unique(&mut loaders, loader);
                }
            }
            let mut categories = Vec::new();
            for category in &hit.categories {
                if let Some(shared) = shared_category(&category.slug) {
                    push_unique(&mut categories, shared);
                }
            }
            ModpackListItem {
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
                updated_at: hit.date_modified,
                loaders,
                categories,
            }
        })
        .collect();
    Ok(SourcePage { items, total })
}

/// Category ids by slug, from `GET /v1/categories?classId=4471`.
pub(super) fn parse_categories(body: &str) -> Result<HashMap<String, u64>> {
    let parsed: CategoriesResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    Ok(parsed
        .data
        .into_iter()
        .map(|category| (category.slug, category.id))
        .collect())
}

/// CurseForge's modpack category for each shared category; `None` when it has no match.
pub(super) fn category_slug(category: ModpackCategory) -> Option<&'static str> {
    match category {
        ModpackCategory::Adventure => Some("adventure-and-rpg"),
        ModpackCategory::Challenging => Some("hardcore"),
        ModpackCategory::Combat => Some("combat-pvp"),
        ModpackCategory::KitchenSink => Some("extra-large"),
        ModpackCategory::Lightweight => Some("small-light"),
        ModpackCategory::Magic => Some("magic"),
        ModpackCategory::Multiplayer => Some("multiplayer"),
        ModpackCategory::Optimization => None,
        ModpackCategory::Quests => Some("quests"),
        ModpackCategory::Technology => Some("tech"),
    }
}

fn loader_from_type(id: u64) -> Option<ModpackLoader> {
    ModpackLoader::ALL
        .into_iter()
        .find(|loader| mod_loader_type(*loader) == id.to_string())
}

/// Loader names CurseForge mixes into a file's `gameVersions`.
fn loader_from_name(name: &str) -> Option<ModpackLoader> {
    match name.to_ascii_lowercase().as_str() {
        "fabric" => Some(ModpackLoader::Fabric),
        "forge" => Some(ModpackLoader::Forge),
        "neoforge" => Some(ModpackLoader::Neoforge),
        "quilt" => Some(ModpackLoader::Quilt),
        _ => None,
    }
}

fn shared_category(slug: &str) -> Option<ModpackCategory> {
    ModpackCategory::ALL
        .into_iter()
        .find(|shared| category_slug(*shared) == Some(slug))
}

/// `ModLoaderType` values from the CurseForge API reference.
pub(super) fn mod_loader_type(loader: ModpackLoader) -> &'static str {
    match loader {
        ModpackLoader::Forge => "1",
        ModpackLoader::Fabric => "4",
        ModpackLoader::Quilt => "5",
        ModpackLoader::Neoforge => "6",
    }
}

/// `ModsSearchSortField` values. CurseForge has no text relevance sort, so
/// relevance uses its popularity ranking.
pub(super) fn sort_field(sort: ModpackSort) -> &'static str {
    match sort {
        ModpackSort::Downloads => "6",
        ModpackSort::Updated => "3",
        ModpackSort::Relevance => "2",
    }
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
    let mut loaders = Vec::new();
    let mut game_versions = Vec::new();
    for index in &project.latest_files_indexes {
        if let Some(loader) = index.mod_loader.and_then(loader_from_type) {
            push_unique(&mut loaders, loader);
        }
        if let Some(version) = &index.game_version {
            push_unique(&mut game_versions, version.clone());
        }
    }
    let mut categories = Vec::new();
    for category in &project.categories {
        if let Some(shared) = shared_category(&category.slug) {
            push_unique(&mut categories, shared);
        }
    }
    let links = project.links.unwrap_or(Links {
        website_url: None,
        wiki_url: None,
        issues_url: None,
        source_url: None,
    });
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
        followers: None,
        icon_url: project.logo.and_then(|logo| logo.url),
        categories,
        loaders,
        game_versions,
        license: None,
        published_at: project.date_created,
        updated_at: project.date_modified,
        gallery: project
            .screenshots
            .into_iter()
            .filter(|screenshot| screenshot.url.starts_with("https://"))
            .map(|screenshot| ModpackImage {
                url: screenshot.url,
                title: screenshot.title.filter(|title| !title.is_empty()),
            })
            .collect(),
        project_url: https_only(links.website_url),
        links: ModpackLinks {
            source: https_only(links.source_url),
            issues: https_only(links.issues_url),
            wiki: https_only(links.wiki_url),
            discord: None,
        },
        description_html: sanitize_html(&description.data),
    })
}

/// One page of a pack's files; `releaseType` 1/2/3 is release/beta/alpha.
pub(super) fn parse_versions(body: &str) -> Result<FilesPage> {
    let parsed: FilesResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let total = parsed
        .pagination
        .map(|pagination| pagination.total_count)
        .unwrap_or(0);
    let items = parsed
        .data
        .into_iter()
        .map(|file| {
            let mut loaders = Vec::new();
            let mut game_versions = Vec::new();
            for entry in &file.game_versions {
                if let Some(loader) = loader_from_name(entry) {
                    push_unique(&mut loaders, loader);
                } else if entry.starts_with(|ch: char| ch.is_ascii_digit()) {
                    push_unique(&mut game_versions, entry.clone());
                }
            }
            ModpackVersionItem {
                id: file.id.to_string(),
                name: if file.display_name.is_empty() {
                    file.file_name
                } else {
                    file.display_name
                },
                game_versions,
                loaders,
                version_type: match file.release_type {
                    Some(2) => ModpackVersionType::Beta,
                    Some(3) => ModpackVersionType::Alpha,
                    _ => ModpackVersionType::Release,
                },
                published_at: file.file_date,
                downloads: file.download_count,
            }
        })
        .collect();
    Ok(FilesPage { items, total })
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
        return Err(status_error(status, label));
    }
    Ok(body)
}

/// CurseForge answers 401/403 when the `x-api-key` header is not a valid key.
pub(super) fn status_error(status: u16, label: &str) -> CoreError {
    match status {
        401 | 403 => CoreError::CurseforgeKeyRejected,
        _ => CoreError::ModpackMeta {
            detail: format!("{label}_http_{status}"),
        },
    }
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
        return Err(status_error(status, "curseforge_api"));
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
    fn reads_the_last_update_of_each_hit() {
        let page = parse_search(
            r#"{"data":[{"id":1,"name":"A","summary":"","downloadCount":1,"dateModified":"2026-11-08T00:51:06.56Z"}]}"#,
        )
        .expect("ok");
        assert_eq!(
            page.items[0].updated_at.as_deref(),
            Some("2026-11-08T00:51:06.56Z")
        );
    }

    #[test]
    fn reads_loaders_and_shared_categories_of_each_hit() {
        let page = parse_search(
            r#"{"data":[{"id":1,"name":"A","summary":"","downloadCount":1,
              "categories":[{"slug":"extra-large"},{"slug":"exploration"},{"slug":"tech"}],
              "latestFilesIndexes":[{"gameVersion":"1.21.1","modLoader":6},{"gameVersion":"1.20.1"},{"gameVersion":"1.20.1","modLoader":6},{"gameVersion":"1.12.2","modLoader":1},{"gameVersion":"1.7.10","modLoader":2}]}]}"#,
        )
        .expect("ok");
        assert_eq!(
            page.items[0].loaders,
            vec![ModpackLoader::Neoforge, ModpackLoader::Forge]
        );
        assert_eq!(
            page.items[0].categories,
            vec![ModpackCategory::KitchenSink, ModpackCategory::Technology]
        );
    }

    #[test]
    fn maps_category_slugs_to_ids() {
        let ids = parse_categories(
            r#"{"data":[{"id":4472,"slug":"tech","name":"Tech","classId":4471},{"id":4473,"slug":"magic","name":"Magic","classId":4471}]}"#,
        )
        .expect("ok");
        assert_eq!(ids.get("tech"), Some(&4472));
        assert_eq!(ids.get("magic"), Some(&4473));
    }

    #[test]
    fn every_shared_category_but_optimization_has_a_curseforge_match() {
        assert_eq!(category_slug(ModpackCategory::Technology), Some("tech"));
        assert_eq!(
            category_slug(ModpackCategory::KitchenSink),
            Some("extra-large")
        );
        assert_eq!(category_slug(ModpackCategory::Optimization), None);
    }

    #[test]
    fn a_rejected_key_is_its_own_error() {
        for status in [401, 403] {
            assert!(matches!(
                status_error(status, "curseforge_search"),
                CoreError::CurseforgeKeyRejected
            ));
        }
    }

    #[test]
    fn other_failures_keep_the_request_label() {
        match status_error(500, "curseforge_search") {
            CoreError::ModpackMeta { detail } => assert_eq!(detail, "curseforge_search_http_500"),
            other => panic!("unexpected error: {other:?}"),
        }
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
            "categories":[{"name":"Kitchen Sink","slug":"extra-large"},{"name":"Exploration","slug":"exploration"}],
            "dateCreated":"2024-06-25T09:47:22.51Z",
            "dateModified":"2026-10-01T00:00:00Z",
            "screenshots":[{"url":"https://media.forgecdn.net/s.png","title":"Spawn"},{"url":"http://insecure.example/x.png"}],
            "latestFilesIndexes":[{"gameVersion":"1.21.1","modLoader":6},{"gameVersion":"1.20.1"},{"gameVersion":"1.21.1","modLoader":6}],
            "links":{"websiteUrl":"https://www.curseforge.com/minecraft/modpacks/all-the-mods-9","wikiUrl":"","issuesUrl":"https://github.com/AllTheMods/ATM-10/issues/","sourceUrl":"http://insecure.example/src"}
          }
        }"#;
        let description = r#"{"data":"<p>Hello</p><img src=\"x\" onerror=\"alert(1)\">"}"#;
        let details = parse_mod(project, description).expect("ok");
        assert_eq!(details.id, "381671");
        assert_eq!(details.author, "ATMTeam");
        assert_eq!(details.categories, vec![ModpackCategory::KitchenSink]);
        assert_eq!(details.loaders, vec![ModpackLoader::Neoforge]);
        assert_eq!(details.game_versions, vec!["1.21.1", "1.20.1"]);
        assert_eq!(details.followers, None);
        assert_eq!(details.license, None);
        assert_eq!(
            details.published_at.as_deref(),
            Some("2024-06-25T09:47:22.51Z")
        );
        assert_eq!(
            details.gallery,
            vec![ModpackImage {
                url: "https://media.forgecdn.net/s.png".into(),
                title: Some("Spawn".into())
            }]
        );
        assert_eq!(
            details.links,
            ModpackLinks {
                source: None,
                issues: Some("https://github.com/AllTheMods/ATM-10/issues/".into()),
                wiki: None,
                discord: None,
            }
        );
        assert_eq!(
            details.project_url.as_deref(),
            Some("https://www.curseforge.com/minecraft/modpacks/all-the-mods-9")
        );
        assert!(details.description_html.contains("<p>Hello</p>"));
        assert!(!details.description_html.contains("onerror"));
    }

    #[test]
    fn parses_a_files_page_with_type_date_and_downloads() {
        let page = parse_versions(
            r#"{"data":[
              {"id":9431424,"displayName":"All the Mods 10-8.5","fileName":"atm10.zip","releaseType":1,
               "fileDate":"2026-11-04T07:16:05.953Z","downloadCount":72327,"gameVersions":["1.21.1","NeoForge"]},
              {"id":2,"displayName":"","fileName":"beta.zip","releaseType":2,"gameVersions":["Forge","1.20.1","Client"]},
              {"id":3,"displayName":"Alpha","releaseType":3,"gameVersions":[]}
            ],"pagination":{"index":0,"pageSize":50,"resultCount":3,"totalCount":148}}"#,
        )
        .expect("ok");
        assert_eq!(page.total, 148);
        let first = &page.items[0];
        assert_eq!(first.id, "9431424");
        assert_eq!(first.game_versions, vec!["1.21.1"]);
        assert_eq!(first.loaders, vec![ModpackLoader::Neoforge]);
        assert_eq!(first.version_type, ModpackVersionType::Release);
        assert_eq!(
            first.published_at.as_deref(),
            Some("2026-11-04T07:16:05.953Z")
        );
        assert_eq!(first.downloads, 72327);
        assert_eq!(page.items[1].name, "beta.zip");
        assert_eq!(page.items[1].loaders, vec![ModpackLoader::Forge]);
        assert_eq!(page.items[1].version_type, ModpackVersionType::Beta);
        assert_eq!(page.items[2].version_type, ModpackVersionType::Alpha);
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
