use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use zip::ZipArchive;

use super::api::{RemoteFile, RemoteMod, resolve_remote};
use super::cdn::{canonical_download_url, cdn_client, select_cdn_url};
use super::{CURSEFORGE_INDEX_FILE, curseforge_api_key};
use crate::download::{DEFAULT_RETRY, DownloadProgress, DownloadTask, download_all};
use crate::instance::{InstanceEntry, create_instance};
use crate::modpacks::MODPACK_INDEX_FILE;
use crate::modpacks::file_hashes::{ensure_md5, prepare_destination_sha1_md5};
use crate::modpacks::path_safety::{path_component_allowed, safe_instance_path};
use crate::settings::{GlobalLaunch, download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};

/// Names that must not be overwritten at the instance root by pack content.
const PROTECTED_ROOT_NAMES: [&str; 3] =
    ["instance.json", MODPACK_INDEX_FILE, CURSEFORGE_INDEX_FILE];

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
    let file_ids = manifest_files.iter().map(|file| file.file_id).collect();
    let (remote, mods) = resolve_remote(client, key, file_ids).await?;
    plan_files(manifest_files, &remote, &mods)
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
        let _ = safe_instance_path(Path::new("."), &path, &PROTECTED_ROOT_NAMES)?;
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
        let _ = safe_instance_path(Path::new("."), &relative, &PROTECTED_ROOT_NAMES)?;
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
        let destination = safe_instance_path(instance_dir, relative, &PROTECTED_ROOT_NAMES)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}

fn pending_task(file: &IndexFile, instance_dir: &Path) -> Result<Option<PendingDownload>> {
    let destination = safe_instance_path(instance_dir, &file.path, &PROTECTED_ROOT_NAMES)?;
    let url = select_cdn_url(&file.downloads)?;
    let sha1 = present_hash(&file.hashes.sha1);
    let md5 = file
        .hashes
        .md5
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if !prepare_destination_sha1_md5(&destination, &sha1, md5.as_deref())? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::api::remote_mods_from_body;
    use std::io::Write;
    use std::path::PathBuf;
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

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
        let ok = remote_at(1, 9, "https://edge.forgecdn.net/files/1/1/ok.jar");
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
            &[ok, hidden],
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
            "https://github.com/a/b/releases/download/v/a.jar",
            "https://cdn.modrinth.com/data/a.jar",
            "https://edge.forgecdn.net.evil.example/a.jar",
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

        let launcher_file = IndexFile {
            path: "licht-curseforge.json".into(),
            ..index_file("mods/a.jar", "https://edge.forgecdn.net/files/1/2/a.jar")
        };
        let error = pending_task(&launcher_file, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let github = index_file(
            "mods/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
        );
        let error = pending_task(&github, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::CurseforgeDownloadHost));

        let keyed = index_file(
            "mods/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar?api-key=secret",
        );
        let root = temp_dir("pick");
        let task = pending_task(&keyed, &root)
            .expect("task")
            .expect("download");
        assert!(task.task.url.contains("edge.forgecdn.net"));
        assert!(!task.task.url.contains("secret"));
        let _ = fs::remove_dir_all(&root);
    }

    fn index_file(path: &str, url: &str) -> IndexFile {
        IndexFile {
            path: path.into(),
            downloads: vec![url.into()],
            hashes: IndexHashes::default(),
            file_size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "a.jar".into(),
            name: "A".into(),
            required: true,
        }
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
}
