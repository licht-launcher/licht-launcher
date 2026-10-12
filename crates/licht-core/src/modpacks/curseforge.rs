use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;
use url::Url;
use zip::ZipArchive;

use crate::download::{DEFAULT_RETRY, DownloadProgress, DownloadTask, download_all};
use crate::instance::{InstanceEntry, create_instance};
use crate::modpacks::types::{ModpackListItem, ModpackSource, ModpackVersionItem};
use crate::settings::{GlobalLaunch, download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};

const SEARCH_URL: &str = "https://api.curseforge.com/v1/mods/search";
const MOD_URL: &str = "https://api.curseforge.com/v1/mods";
const FILES_URL: &str = "https://api.curseforge.com/v1/mods/files";
const GAME_ID: &str = "432";
const CLASS_ID: &str = "4471";

/// Ids per `POST /v1/mods/files` and `POST /v1/mods` call.
/// The published schema does not state a maximum. 100 is a batch size other
/// CurseForge clients send successfully.
const FILE_BATCH: usize = 100;

/// Same cap as reqwest's default redirect limit.
const MAX_CDN_REDIRECTS: usize = 10;

pub const CURSEFORGE_API_KEY_ENV: &str = "LICHT_CURSEFORGE_API_KEY";

/// Resolved CurseForge files for one instance, parallel to `licht-modpack.json`.
pub const CURSEFORGE_INDEX_FILE: &str = "licht-curseforge.json";

/// Hosts the CurseForge API returns for file bytes.
///
/// Match is exact and HTTPS-only, including every redirect. `cdn.modrinth.com`
/// is absent on purpose: the `.mrpack` allowlist stays Modrinth-only, so a
/// forgecdn URL inside an `.mrpack` is still refused there.
const CURSEFORGE_CDN_HOSTS: &[&str] = &[
    "edge.forgecdn.net",
    "mediafilez.forgecdn.net",
    "media.forgecdn.net",
];

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
        return Err(download_url_status_error(status, &body));
    }
    let download_url = parse_pack_download_url(&body)?;
    let bytes = fetch_cdn_bytes(&key, &download_url).await?;
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
        Some(Value::String(url)) => canonical_download_url(url),
        _ => Err(CoreError::ModpackMeta {
            detail: "curseforge_download_url_missing".into(),
        }),
    }
}

/// Creates an instance from a CurseForge pack zip.
///
/// Resolves `manifest.json` files through the CurseForge API, writes
/// [`CURSEFORGE_INDEX_FILE`], and extracts overrides. Download of those files
/// happens in [`install_curseforge_files`]. Mods with a null `downloadUrl` are
/// not fetched from a guessed CDN path; import fails before the instance exists
/// and the error lists them.
pub async fn import_curseforge_pack(
    client: &Client,
    instances_dir: &Path,
    name: &str,
    bytes: &[u8],
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let manifest = manifest_from_zip(bytes)?;
    let plan = if manifest.files.is_empty() {
        Plan::default()
    } else {
        let key = curseforge_api_key()?;
        resolve_plan(client, &key, &manifest.files).await?
    };
    import_planned(instances_dir, name, bytes, &manifest, &plan, template)
}

/// Downloads files recorded in [`CURSEFORGE_INDEX_FILE`].
///
/// No-op when the index is missing. Each request and redirect must be HTTPS on
/// a CurseForge CDN host. The API key is a default header on that client, not
/// part of the stored URL.
pub async fn install_curseforge_files(
    game: &SharedCache,
    settings: &SharedCache,
    instance_dir: &Path,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let index_path = instance_dir.join(CURSEFORGE_INDEX_FILE);
    if !index_path.is_file() {
        return Ok(());
    }
    let body = fs::read_to_string(&index_path)?;
    let index: CurseforgeIndex = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    let mut pending = Vec::new();
    for file in &index.files {
        if let Some(item) = pending_task(file, instance_dir)? {
            pending.push(item);
        }
    }
    if pending.is_empty() {
        let _ = progress.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        });
        return Ok(());
    }
    let key = curseforge_api_key()?;
    let client = cdn_client(&key)?;
    let tasks: Vec<DownloadTask> = pending.iter().map(|item| item.task.clone()).collect();
    download_all(
        &client,
        &tasks,
        shared_transfer_limits(),
        download_concurrency(game, settings)?,
        write_concurrency(game, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await?;
    for item in &pending {
        if let Some(expected) = &item.md5 {
            ensure_md5(&item.task.destination, expected)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Manifest {
    game_version: String,
    files: Vec<ManifestFile>,
    overrides_dir: String,
}

#[derive(Debug)]
struct ManifestFile {
    project_id: u64,
    file_id: u64,
    required: bool,
}

#[derive(Debug, Default)]
struct Plan {
    files: Vec<PlannedFile>,
    blocked: Vec<BlockedMod>,
}

#[derive(Debug)]
struct PlannedFile {
    path: String,
    url: String,
    sha1: Option<String>,
    md5: Option<String>,
    size: u64,
    project_id: u64,
    file_id: u64,
    file_name: String,
    name: String,
    required: bool,
}

#[derive(Debug)]
struct BlockedMod {
    name: String,
    file_name: String,
}

#[derive(Debug)]
struct RemoteFile {
    file_id: u64,
    mod_id: u64,
    file_name: String,
    display_name: String,
    size: u64,
    sha1: Option<String>,
    md5: Option<String>,
    /// `None` when `downloadUrl` is null or empty: the author disabled distribution.
    download_url: Option<String>,
}

#[derive(Debug)]
struct RemoteMod {
    id: u64,
    name: String,
    class_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseforgeIndex {
    source: String,
    files: Vec<IndexFile>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    path: String,
    downloads: Vec<String>,
    hashes: IndexHashes,
    file_size: u64,
    project_id: u64,
    file_id: u64,
    file_name: String,
    name: String,
    required: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct IndexHashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    md5: Option<String>,
}

#[derive(Debug)]
struct PendingDownload {
    task: DownloadTask,
    md5: Option<String>,
}

enum Algorithm {
    Sha1,
    Md5,
}

enum RedirectHop {
    Follow,
    Blocked,
    TooMany,
}

fn manifest_from_zip(bytes: &[u8]) -> Result<Manifest> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let mut file = archive
        .by_name("manifest.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let mut body = String::new();
    file.read_to_string(&mut body)
        .map_err(|_| CoreError::ModpackArchive)?;
    parse_manifest(&body)
}

fn parse_manifest(body: &str) -> Result<Manifest> {
    let value: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let game_version = value
        .pointer("/minecraft/version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .ok_or(CoreError::ModpackArchive)?
        .to_string();
    let files = match value.get("files") {
        None => Vec::new(),
        Some(Value::Array(items)) => parse_file_entries(items)?,
        Some(_) => return Err(CoreError::ModpackArchive),
    };
    let overrides_dir = value
        .get("overrides")
        .and_then(Value::as_str)
        .unwrap_or("overrides")
        .to_string();
    Ok(Manifest {
        game_version,
        files,
        overrides_dir,
    })
}

fn parse_file_entries(items: &[Value]) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();
    for item in items {
        let project_id = item
            .get("projectID")
            .and_then(Value::as_u64)
            .ok_or(CoreError::ModpackArchive)?;
        let file_id = item
            .get("fileID")
            .and_then(Value::as_u64)
            .ok_or(CoreError::ModpackArchive)?;
        let required = item
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        files.push(ManifestFile {
            project_id,
            file_id,
            required,
        });
    }
    Ok(files)
}

async fn resolve_plan(client: &Client, key: &str, manifest_files: &[ManifestFile]) -> Result<Plan> {
    let file_ids = unique_sorted(manifest_files.iter().map(|file| file.file_id).collect());
    if file_ids.is_empty() {
        return Ok(Plan::default());
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
    plan_files(manifest_files, &remote, &mods)
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
fn remote_files_from_body(body: &str) -> Result<Vec<RemoteFile>> {
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
fn remote_mods_from_body(body: &str) -> Result<Vec<RemoteMod>> {
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

fn plan_files(
    manifest_files: &[ManifestFile],
    remote_files: &[RemoteFile],
    mods: &[RemoteMod],
) -> Result<Plan> {
    let mut by_file: HashMap<u64, &RemoteFile> = HashMap::new();
    for file in remote_files {
        by_file.entry(file.file_id).or_insert(file);
    }
    let mut by_mod: HashMap<u64, &RemoteMod> = HashMap::new();
    for project in mods {
        by_mod.entry(project.id).or_insert(project);
    }
    let mut plan = Plan::default();
    let mut seen_files = HashSet::new();
    let mut seen_paths = HashSet::new();
    for entry in manifest_files {
        if !seen_files.insert(entry.file_id) {
            continue;
        }
        let Some(remote) = by_file.get(&entry.file_id) else {
            return Err(CoreError::CurseforgeFileMissing {
                file_id: entry.file_id,
            });
        };
        let project = by_mod.get(&remote.mod_id).copied();
        let name = mod_label(
            project.map(|item| item.name.as_str()),
            &remote.display_name,
            &remote.file_name,
            entry.project_id,
        );
        let Some(raw_url) = remote.download_url.as_deref() else {
            plan.blocked.push(BlockedMod {
                name,
                file_name: one_line(&remote.file_name),
            });
            continue;
        };
        let file_name = safe_download_name(&remote.file_name)?;
        let url = canonical_download_url(raw_url)?;
        let directory = content_directory(project.and_then(|item| item.class_id));
        let path = format!("{directory}/{file_name}");
        if !seen_paths.insert(path.clone()) {
            return Err(CoreError::ModpackArchive);
        }
        // Reject the joined path here, before anything is written.
        let _ = safe_instance_path(Path::new("."), &path)?;
        plan.files.push(PlannedFile {
            path,
            url,
            sha1: remote.sha1.clone(),
            md5: remote.md5.clone(),
            size: remote.size,
            project_id: entry.project_id,
            file_id: entry.file_id,
            file_name,
            name,
            required: entry.required,
        });
    }
    Ok(plan)
}

/// CurseForge Minecraft class ids. Unknown classes land in `mods`.
/// World zips are stored under `saves/` and are not extracted.
fn content_directory(class_id: Option<u64>) -> &'static str {
    match class_id {
        Some(12) => "resourcepacks",
        Some(17) => "saves",
        Some(6552) => "shaderpacks",
        Some(6945) => "datapacks",
        _ => "mods",
    }
}

fn mod_label(
    project_name: Option<&str>,
    display_name: &str,
    file_name: &str,
    project_id: u64,
) -> String {
    let chosen = if let Some(name) = project_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        name.to_string()
    } else if !display_name.trim().is_empty() {
        display_name.trim().to_string()
    } else if !file_name.trim().is_empty() {
        file_name.trim().to_string()
    } else {
        format!("project {project_id}")
    };
    one_line(&chosen)
}

fn one_line(value: &str) -> String {
    value
        .replace(['\n', '\r', '\t'], " ")
        .chars()
        .take(120)
        .collect()
}

fn format_blocked(blocked: &[BlockedMod]) -> String {
    let mut items: Vec<&BlockedMod> = blocked.iter().collect();
    items.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    items
        .iter()
        .map(|item| {
            if item.file_name.is_empty() {
                item.name.clone()
            } else {
                format!("{} ({})", item.name, item.file_name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn import_planned(
    instances_dir: &Path,
    name: &str,
    bytes: &[u8],
    manifest: &Manifest,
    plan: &Plan,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    if !plan.blocked.is_empty() {
        return Err(CoreError::CurseforgeDistributionDenied {
            mods: format_blocked(&plan.blocked),
        });
    }
    let prefixes = override_prefixes(&manifest.overrides_dir);
    let entries = list_override_files(bytes, &prefixes)?;
    let entry = create_instance(instances_dir, name, &manifest.game_version, template)?;
    let instance_dir = instances_dir.join(&entry.folder);
    extract_override_files(bytes, &instance_dir, &entries)?;
    write_index(&instance_dir, plan)?;
    Ok(entry)
}

fn write_index(instance_dir: &Path, plan: &Plan) -> Result<()> {
    let index = CurseforgeIndex {
        source: "curseforge".to_string(),
        files: plan
            .files
            .iter()
            .map(|file| IndexFile {
                path: file.path.clone(),
                downloads: vec![file.url.clone()],
                hashes: IndexHashes {
                    sha1: file.sha1.clone(),
                    md5: file.md5.clone(),
                },
                file_size: file.size,
                project_id: file.project_id,
                file_id: file.file_id,
                file_name: file.file_name.clone(),
                name: file.name.clone(),
                required: file.required,
            })
            .collect(),
    };
    let body = serde_json::to_string_pretty(&index).map_err(CoreError::LoaderJson)?;
    fs::write(instance_dir.join(CURSEFORGE_INDEX_FILE), body)?;
    Ok(())
}

fn override_prefixes(overrides_dir: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    if let Some(dir) = safe_overrides_dir(overrides_dir) {
        prefixes.push(dir);
    }
    if !prefixes.iter().any(|item| item == "overrides") {
        prefixes.push("overrides".to_string());
    }
    if !prefixes.iter().any(|item| item == "override") {
        prefixes.push("override".to_string());
    }
    prefixes
}

fn safe_overrides_dir(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('/');
    if !path_component_allowed(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

fn list_override_files(bytes: &[u8], prefixes: &[String]) -> Result<Vec<(usize, String)>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let name = file.name().replace('\\', "/");
        let Some(relative) = relative_override(&name, prefixes) else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        if file.enclosed_name().is_none() {
            return Err(CoreError::ModpackArchive);
        }
        let relative = relative.to_string();
        let _ = safe_instance_path(Path::new("."), &relative)?;
        files.push((index, relative));
    }
    Ok(files)
}

fn relative_override<'a>(name: &'a str, prefixes: &[String]) -> Option<&'a str> {
    for prefix in prefixes {
        let with_slash = format!("{prefix}/");
        if let Some(relative) = name.strip_prefix(&with_slash) {
            return Some(relative);
        }
    }
    None
}

fn extract_override_files(
    bytes: &[u8],
    instance_dir: &Path,
    files: &[(usize, String)],
) -> Result<()> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    for (index, relative) in files {
        let mut file = archive
            .by_index(*index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let destination = safe_instance_path(instance_dir, relative)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}

fn pending_task(file: &IndexFile, instance_dir: &Path) -> Result<Option<PendingDownload>> {
    let destination = safe_instance_path(instance_dir, &file.path)?;
    let url = select_cdn_url(&file.downloads)?;
    let sha1 = present_hash(&file.hashes.sha1);
    let md5 = file
        .hashes
        .md5
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if !prepare_destination(&destination, &sha1, md5.as_deref())? {
        return Ok(None);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Some(PendingDownload {
        task: DownloadTask {
            url,
            destination,
            sha1,
            size: file.file_size,
        },
        md5,
    }))
}

fn present_hash(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
        .unwrap_or("")
        .to_string()
}

fn select_cdn_url(urls: &[String]) -> Result<String> {
    if urls.is_empty() {
        return Err(CoreError::ModpackArchive);
    }
    for url in urls {
        if download_url_allowed(url) {
            return canonical_download_url(url);
        }
    }
    Err(CoreError::CurseforgeDownloadHost)
}

/// `true` when the file still has to be fetched.
///
/// An empty hash is not a match. An existing file is removed so the downloader
/// does not treat "the file exists" as success when SHA1 is missing.
fn prepare_destination(path: &Path, sha1: &str, md5: Option<&str>) -> Result<bool> {
    let has_sha1 = !sha1.is_empty();
    let has_md5 = md5.is_some_and(|value| !value.is_empty());
    if !has_sha1 && !has_md5 {
        if path.is_file() {
            fs::remove_file(path)?;
        }
        return Ok(true);
    }
    let sha1_ok = !has_sha1 || hash_matches(path, sha1, Algorithm::Sha1)?;
    let md5_ok = !has_md5 || hash_matches(path, md5.unwrap_or(""), Algorithm::Md5)?;
    if sha1_ok && md5_ok && path.is_file() {
        return Ok(false);
    }
    if path.is_file() && (!has_sha1 || sha1_ok) {
        fs::remove_file(path)?;
    }
    Ok(true)
}

fn ensure_md5(path: &Path, expected: &str) -> Result<()> {
    let actual = file_hash(path, Algorithm::Md5)?.unwrap_or_default();
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    let _ = fs::remove_file(path);
    Err(CoreError::Md5Mismatch {
        expected: expected.to_string(),
        actual,
    })
}

fn hash_matches(path: &Path, expected: &str, algorithm: Algorithm) -> Result<bool> {
    let Some(actual) = file_hash(path, algorithm)? else {
        return Ok(false);
    };
    Ok(actual.eq_ignore_ascii_case(expected))
}

fn file_hash(path: &Path, algorithm: Algorithm) -> Result<Option<String>> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut buffer = [0_u8; 64 * 1024];
    match algorithm {
        Algorithm::Sha1 => {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(Some(hex_encode(hasher.finalize().as_slice())))
        }
        Algorithm::Md5 => {
            use md5::Digest;
            let mut hasher = md5::Md5::new();
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(Some(hex_encode(hasher.finalize().as_slice())))
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn safe_download_name(file_name: &str) -> Result<String> {
    let name = file_name.trim();
    if name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || !path_component_allowed(name)
    {
        return Err(CoreError::ModpackArchive);
    }
    Ok(name.to_string())
}

fn safe_instance_path(instance_dir: &Path, relative: &str) -> Result<PathBuf> {
    let normalized = relative.replace('\\', "/");
    // `C:foo` is not absolute on Windows, but joining it replaces the drive.
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized.contains(':')
    {
        return Err(CoreError::ModpackArchive);
    }
    let mut destination = instance_dir.to_path_buf();
    let mut depth = 0usize;
    for part in normalized.split('/') {
        if !path_component_allowed(part) {
            return Err(CoreError::ModpackArchive);
        }
        if depth == 0 && is_launcher_file(part) {
            return Err(CoreError::ModpackArchive);
        }
        destination.push(part);
        depth += 1;
    }
    if depth == 0 {
        return Err(CoreError::ModpackArchive);
    }
    Ok(destination)
}

fn path_component_allowed(part: &str) -> bool {
    if part.is_empty() || part == "." || part == ".." {
        return false;
    }
    // Windows strips trailing dots and spaces, so `instance.json.` can alias the profile.
    if part.ends_with('.') || part.ends_with(' ') {
        return false;
    }
    !is_windows_reserved(part)
}

fn is_launcher_file(name: &str) -> bool {
    name.eq_ignore_ascii_case("instance.json")
        || name.eq_ignore_ascii_case(super::MODPACK_INDEX_FILE)
        || name.eq_ignore_ascii_case(CURSEFORGE_INDEX_FILE)
}

/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, including `CON.txt`.
fn is_windows_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part);
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ((upper.starts_with("COM") || upper.starts_with("LPT"))
        && upper.len() == 4
        && upper.as_bytes()[3].is_ascii_digit()
        && upper.as_bytes()[3] != b'0')
}

fn download_url_allowed(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https" || url.port().is_some() {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(url::Host::Domain(host)) = url.host() else {
        return false;
    };
    CURSEFORGE_CDN_HOSTS
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
}

fn canonical_download_url(raw: &str) -> Result<String> {
    if !download_url_allowed(raw) {
        return Err(CoreError::CurseforgeDownloadHost);
    }
    let mut url = Url::parse(raw).map_err(|_| CoreError::CurseforgeDownloadHost)?;
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            !key.eq_ignore_ascii_case("api-key") && !key.eq_ignore_ascii_case("api_key")
        })
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    if !kept.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in &kept {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

/// `previous_len` counts URLs already requested, including the first one.
fn redirect_hop(next: &str, previous_len: usize) -> RedirectHop {
    if previous_len > MAX_CDN_REDIRECTS {
        return RedirectHop::TooMany;
    }
    if download_url_allowed(next) {
        RedirectHop::Follow
    } else {
        RedirectHop::Blocked
    }
}

fn cdn_client(key: &str) -> Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    let value =
        reqwest::header::HeaderValue::from_str(key).map_err(|_| CoreError::ModpackMeta {
            detail: "curseforge_api_key_invalid".into(),
        })?;
    let _ = headers.insert(reqwest::header::HeaderName::from_static("x-api-key"), value);
    Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::custom(
            |attempt| match redirect_hop(attempt.url().as_str(), attempt.previous().len()) {
                RedirectHop::Follow => attempt.follow(),
                RedirectHop::Blocked => attempt.error("modpack redirect target is not allowed"),
                RedirectHop::TooMany => attempt.error("too many modpack redirects"),
            },
        ))
        .build()
        .map_err(CoreError::from)
}

async fn fetch_cdn_bytes(key: &str, url: &str) -> Result<Vec<u8>> {
    let url = canonical_download_url(url)?;
    let client = cdn_client(key)?;
    let response = client.get(url).send().await?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_cdn_http_{status}"),
        });
    }
    Ok(response.bytes().await?.to_vec())
}

fn unique_sorted(mut ids: Vec<u64>) -> Vec<u64> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

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
    fn parses_manifest_files_and_defaults_required() {
        let manifest = parse_manifest(
            r#"{"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}"#,
        )
        .expect("manifest");
        assert_eq!(manifest.game_version, "1.20.1");
        assert_eq!(manifest.overrides_dir, "overrides");
        assert!(manifest.files[0].required);
        assert_eq!(manifest.files[0].file_id, 2);
    }

    #[test]
    fn rejects_a_manifest_without_a_minecraft_version() {
        let error = parse_manifest(r#"{"files":[]}"#).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
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
    fn places_files_by_class_and_strips_api_keys_from_urls() {
        let mods = remote_mods_from_body(
            r#"{"data":[
              {"id":1,"name":"Sodium","classId":6},
              {"id":2,"name":"Faithful","classId":12},
              {"id":3,"name":"Complementary","classId":6552},
              {"id":4,"name":"Data","classId":6945},
              {"id":5,"name":"World","classId":17}
            ]}"#,
        )
        .expect("mods");
        let remote = vec![
            remote_at(
                1,
                1,
                "https://edge.forgecdn.net/files/1/1/sodium.jar?api-key=super-secret",
            ),
            remote_at(2, 2, "https://media.forgecdn.net/files/1/2/faithful.zip"),
            remote_at(3, 3, "https://EDGE.FORGECDN.NET/files/1/3/shader.zip"),
            remote_at(4, 4, "https://mediafilez.forgecdn.net/files/1/4/data.zip"),
            remote_at(5, 5, "https://edge.forgecdn.net:443/files/1/5/world.zip"),
        ];
        let manifest = vec![
            manifest_file(10, 1),
            manifest_file(11, 2),
            manifest_file(12, 3),
            manifest_file(13, 4),
            manifest_file(14, 5),
        ];
        let plan = plan_files(&manifest, &remote, &mods).expect("plan");
        assert!(plan.blocked.is_empty());
        assert_eq!(plan.files[0].path, "mods/1.jar");
        assert_eq!(plan.files[1].path, "resourcepacks/2.jar");
        assert_eq!(plan.files[2].path, "shaderpacks/3.jar");
        assert_eq!(plan.files[3].path, "datapacks/4.jar");
        assert_eq!(plan.files[4].path, "saves/5.jar");
        assert!(!plan.files[0].url.contains("super-secret"));
        assert!(plan.files[0].url.contains("edge.forgecdn.net"));
        assert_eq!(plan.files[0].name, "Sodium");
    }

    #[test]
    fn lists_mods_whose_authors_disabled_distribution() {
        let remote = [remote_at(
            1,
            9,
            "https://edge.forgecdn.net/files/1/1/ok.jar",
        )];
        let mut hidden = remote_at(2, 8, "https://edge.forgecdn.net/files/1/2/hidden.jar");
        hidden.download_url = None;
        hidden.file_name = "hidden.jar".into();
        hidden.display_name = "File Display".into();
        let mods = vec![RemoteMod {
            id: 8,
            name: "Secret Mod".into(),
            class_id: Some(6),
        }];
        let plan = plan_files(
            &[manifest_file(1, 1), manifest_file(2, 2)],
            &[remote[0].clone_file(), hidden],
            &mods,
        )
        .expect("plan");
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.blocked.len(), 1);
        assert_eq!(plan.blocked[0].name, "Secret Mod");
        assert_eq!(format_blocked(&plan.blocked), "Secret Mod (hidden.jar)");
    }

    #[test]
    fn rejects_download_hosts_outside_the_curseforge_cdn() {
        for url in [
            "http://edge.forgecdn.net/files/1/2/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
            "https://cdn.modrinth.com/data/a.jar",
            "https://edge.forgecdn.net.evil.example/a.jar",
            "https://evil.edge.forgecdn.net/a.jar",
            "https://user:secret@edge.forgecdn.net/a.jar",
            "https://edge.forgecdn.net:444/a.jar",
            "https://1.2.3.4/a.jar",
        ] {
            let mut remote = remote_at(1, 2, "https://edge.forgecdn.net/files/1/2/a.jar");
            remote.download_url = Some(url.to_string());
            let error = plan_files(&[manifest_file(2, 1)], &[remote], &[]).expect_err(url);
            assert!(
                matches!(error, CoreError::CurseforgeDownloadHost),
                "{url}: {error}"
            );
            assert_eq!(error.to_string(), "curseforge download host is not allowed");
        }
    }

    #[test]
    fn reports_a_manifest_file_the_api_did_not_return() {
        let error = plan_files(&[manifest_file(1, 9)], &[], &[]).unwrap_err();
        assert!(matches!(
            error,
            CoreError::CurseforgeFileMissing { file_id: 9 }
        ));
    }

    #[test]
    fn rejects_file_names_that_leave_the_instance() {
        let mut remote = remote_at(1, 2, "https://edge.forgecdn.net/files/1/2/a.jar");
        remote.file_name = "../evil.jar".into();
        let error = plan_files(&[manifest_file(2, 1)], &[remote], &[]).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
    }

    #[test]
    fn pack_download_url_must_be_a_cdn_host() {
        let url = parse_pack_download_url(
            r#"{"data":"https://edge.forgecdn.net/files/1/2/pack.zip?api-key=secret"}"#,
        )
        .expect("url");
        assert!(!url.contains("secret"));
        assert!(download_url_allowed(&url));
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

    #[test]
    fn follows_only_curseforge_cdn_redirects() {
        for url in [
            "https://edge.forgecdn.net/files/1/2/a.jar",
            "https://MEDIAFILEZ.FORGECDN.NET/files/1/2/a.jar",
            "https://media.forgecdn.net/files/1/2/a.jar",
        ] {
            assert!(matches!(redirect_hop(url, 1), RedirectHop::Follow), "{url}");
            assert!(
                matches!(redirect_hop(url, 10), RedirectHop::Follow),
                "{url}"
            );
        }
        assert!(matches!(
            redirect_hop("https://edge.forgecdn.net/a.jar", 11),
            RedirectHop::TooMany
        ));
        for url in [
            "http://edge.forgecdn.net/a.jar",
            "https://cdn.modrinth.com/data/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
            "https://objects.githubusercontent.com/a.jar",
            "https://evil.example/a.jar",
        ] {
            assert!(
                matches!(redirect_hop(url, 1), RedirectHop::Blocked),
                "{url}"
            );
        }
    }

    #[tokio::test]
    async fn a_live_redirect_to_a_disallowed_url_is_rejected() {
        for location in [
            "https://evil.example/malware.jar",
            "http://edge.forgecdn.net/a.jar",
            "https://cdn.modrinth.com/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
        ] {
            let url = serve_redirect(location).await;
            let client = cdn_client("test-key").expect("client");
            let error = client.get(&url).send().await.expect_err(location);
            assert!(error.is_redirect(), "{location}: {error}");
            let message = error_chain(&error);
            assert!(
                message.contains("modpack redirect target is not allowed"),
                "{location}: {message}"
            );
        }
    }

    #[test]
    fn imports_overrides_and_records_the_resolved_files() {
        let bytes = zip_with(&[
            (
                "manifest.json",
                br#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
            ),
            ("overrides/options.txt", b"fov:90"),
            ("overrides/config/instance.json", b"nested"),
            ("override/legacy.txt", b"old"),
        ]);
        let plan = Plan {
            files: vec![PlannedFile {
                path: "mods/sodium.jar".into(),
                url: "https://edge.forgecdn.net/files/1/2/sodium.jar".into(),
                sha1: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
                md5: None,
                size: 10,
                project_id: 7,
                file_id: 8,
                file_name: "sodium.jar".into(),
                name: "Sodium".into(),
                required: true,
            }],
            blocked: Vec::new(),
        };
        let root = temp_dir("import");
        let instances = root.join("instances");
        fs::create_dir_all(&instances).unwrap();
        let entry = import_planned(
            &instances,
            "Cobble",
            &bytes,
            &sample_manifest(),
            &plan,
            &launch_template(),
        )
        .expect("import");
        let dir = instances.join(&entry.folder);
        assert_eq!(entry.version_id, "1.20.1");
        assert_eq!(
            fs::read_to_string(dir.join("options.txt")).unwrap(),
            "fov:90"
        );
        assert_eq!(fs::read_to_string(dir.join("legacy.txt")).unwrap(), "old");
        assert_eq!(
            fs::read_to_string(dir.join("config").join("instance.json")).unwrap(),
            "nested"
        );
        let profile = fs::read_to_string(dir.join("instance.json")).unwrap();
        assert!(!profile.contains("nested"));
        let index: Value =
            serde_json::from_str(&fs::read_to_string(dir.join(CURSEFORGE_INDEX_FILE)).unwrap())
                .unwrap();
        assert_eq!(index["source"], "curseforge");
        assert_eq!(index["files"][0]["path"], "mods/sodium.jar");
        assert_eq!(
            index["files"][0]["downloads"][0],
            "https://edge.forgecdn.net/files/1/2/sodium.jar"
        );
        assert_eq!(
            index["files"][0]["hashes"]["sha1"],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert!(index["files"][0]["hashes"].get("md5").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn does_not_create_an_instance_when_distribution_is_denied() {
        let bytes = zip_with(&[(
            "manifest.json",
            br#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
        )]);
        let plan = Plan {
            blocked: vec![
                BlockedMod {
                    name: "Sodium".into(),
                    file_name: "sodium.jar".into(),
                },
                BlockedMod {
                    name: "Iris".into(),
                    file_name: "iris.jar".into(),
                },
            ],
            ..Plan::default()
        };
        let root = temp_dir("blocked");
        let instances = root.join("instances");
        fs::create_dir_all(&instances).unwrap();
        let error = import_planned(
            &instances,
            "Cobble",
            &bytes,
            &sample_manifest(),
            &plan,
            &launch_template(),
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "curseforge mods disallow third-party distribution: Iris (iris.jar), Sodium (sodium.jar)"
        );
        assert!(fs::read_dir(&instances).unwrap().next().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_overrides_that_escape_or_replace_launcher_files() {
        for name in [
            "overrides/instance.json",
            "overrides/licht-modpack.json",
            "overrides/licht-curseforge.json",
            "overrides/C:evil.txt",
            "overrides/CON",
            "overrides/mods/CON.txt",
            "overrides/../../outside.txt",
        ] {
            let bytes = zip_with(&[
                (
                    "manifest.json",
                    br#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
                ),
                (name, b"pwned"),
            ]);
            let root = temp_dir("override");
            let instances = root.join("instances");
            fs::create_dir_all(&instances).unwrap();
            let error = import_planned(
                &instances,
                "Cobble",
                &bytes,
                &sample_manifest(),
                &Plan::default(),
                &launch_template(),
            )
            .expect_err(name);
            assert!(
                matches!(error, CoreError::ModpackArchive),
                "{name}: {error}"
            );
            assert!(fs::read_dir(&instances).unwrap().next().is_none(), "{name}");
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn a_file_without_a_hash_is_not_treated_as_installed() {
        let root = temp_dir("no-hash");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        assert!(prepare_destination(&path, "", None).unwrap());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sha1_or_md5_match_skips_the_download_and_a_mismatch_does_not() {
        let root = temp_dir("hash");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        assert!(!prepare_destination(&path, SHA1, None).unwrap());
        assert!(path.exists());
        assert!(!prepare_destination(&path, "", Some(MD5)).unwrap());
        assert!(path.exists());
        assert!(prepare_destination(&path, SHA1, Some("0".repeat(32).as_str())).unwrap());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn md5_mismatch_removes_the_file() {
        let root = temp_dir("md5");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        let error = ensure_md5(&path, &"0".repeat(32)).unwrap_err();
        assert!(matches!(error, CoreError::Md5Mismatch { .. }));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn skips_a_mod_whose_sha1_already_matches() {
        let root = temp_dir("cached");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        let mods = instance.join("mods");
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("a.jar"), b"hello world").unwrap();
        let index = CurseforgeIndex {
            source: "curseforge".into(),
            files: vec![IndexFile {
                path: "mods/a.jar".into(),
                downloads: vec!["https://edge.forgecdn.net/files/1/2/a.jar".into()],
                hashes: IndexHashes {
                    sha1: Some(SHA1.into()),
                    md5: Some(MD5.into()),
                },
                file_size: 11,
                project_id: 1,
                file_id: 2,
                file_name: "a.jar".into(),
                name: "A".into(),
                required: true,
            }],
        };
        fs::write(
            instance.join(CURSEFORGE_INDEX_FILE),
            serde_json::to_string(&index).unwrap(),
        )
        .unwrap();
        let (progress, _incoming) = mpsc::channel(4);
        install_curseforge_files(&cache, &cache, &instance, progress)
            .await
            .expect("cached file should not be downloaded");
        assert_eq!(fs::read(mods.join("a.jar")).unwrap(), b"hello world");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_an_index_url_or_path_outside_the_allowlist() {
        let escape = IndexFile {
            path: "../evil.jar".into(),
            downloads: vec!["https://edge.forgecdn.net/files/1/2/evil.jar".into()],
            hashes: IndexHashes {
                sha1: Some("a".repeat(40)),
                md5: None,
            },
            file_size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "evil.jar".into(),
            name: "Evil".into(),
            required: true,
        };
        let error = pending_task(&escape, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let github = IndexFile {
            path: "mods/a.jar".into(),
            downloads: vec![
                "https://github.com/a/b/releases/download/v/a.jar".into(),
                "https://edge.forgecdn.net/files/1/2/a.jar?api-key=secret".into(),
            ],
            hashes: IndexHashes::default(),
            file_size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "a.jar".into(),
            name: "A".into(),
            required: true,
        };
        let root = temp_dir("pick");
        let task = pending_task(&github, &root).expect("task");
        let task = task.expect("download");
        assert!(task.task.url.contains("edge.forgecdn.net"));
        assert!(!task.task.url.contains("secret"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sorts_file_ids() {
        assert_eq!(unique_sorted(vec![3, 1, 3, 2]), vec![1, 2, 3]);
    }

    fn manifest_file(project_id: u64, file_id: u64) -> ManifestFile {
        ManifestFile {
            project_id,
            file_id,
            required: true,
        }
    }

    fn remote_at(file_id: u64, mod_id: u64, url: &str) -> RemoteFile {
        RemoteFile {
            file_id,
            mod_id,
            file_name: format!("{file_id}.jar"),
            display_name: format!("File {file_id}"),
            size: 4,
            sha1: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
            md5: None,
            download_url: Some(url.to_string()),
        }
    }

    impl RemoteFile {
        fn clone_file(&self) -> Self {
            Self {
                file_id: self.file_id,
                mod_id: self.mod_id,
                file_name: self.file_name.clone(),
                display_name: self.display_name.clone(),
                size: self.size,
                sha1: self.sha1.clone(),
                md5: self.md5.clone(),
                download_url: self.download_url.clone(),
            }
        }
    }

    fn sample_manifest() -> Manifest {
        Manifest {
            game_version: "1.20.1".into(),
            files: Vec::new(),
            overrides_dir: "overrides".into(),
        }
    }

    fn launch_template() -> GlobalLaunch {
        GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: Vec::new(),
            fullscreen: false,
            width: None,
            height: None,
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-curse-{label}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for (name, contents) in files {
                zip.start_file(*name, options).unwrap();
                zip.write_all(contents).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn error_chain(error: &dyn std::error::Error) -> String {
        let mut message = error.to_string();
        let mut source = std::error::Error::source(error);
        while let Some(inner) = source {
            message.push_str(": ");
            message.push_str(&inner.to_string());
            source = inner.source();
        }
        message
    }

    async fn serve_redirect(location: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        let location = location.to_string();
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut incoming = Vec::new();
            let mut buffer = [0_u8; 256];
            loop {
                let Ok(read) = socket.read(&mut buffer).await else {
                    return;
                };
                if read == 0 {
                    return;
                }
                incoming.extend_from_slice(&buffer[..read]);
                if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let body = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = socket.write_all(body.as_bytes()).await;
        });
        format!("http://{address}/file.jar")
    }
}
