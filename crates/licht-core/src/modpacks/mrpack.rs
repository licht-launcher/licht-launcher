use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use zip::ZipArchive;

use crate::download::{DEFAULT_RETRY, DownloadProgress, DownloadTask, download_all};
use crate::instance::{InstanceEntry, create_instance, update_instance_version};
use crate::loaders::fabric::{self, install_fabric_like};
use crate::loaders::forge::{self, install_forge_like};
use crate::loaders::LoaderChannel;
use crate::settings::{GlobalLaunch, download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};

pub const MODPACK_INDEX_FILE: &str = "licht-modpack.json";

/// Extracts a Modrinth `.mrpack` (zip) into a new instance folder.
///
/// Applies `overrides/` and `client-overrides/` into the instance root and
/// records the full index in [`MODPACK_INDEX_FILE`] for the download pass.
pub fn import_mrpack_bytes(
    instances_dir: &Path,
    name: &str,
    bytes: &[u8],
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| CoreError::ModpackArchive)?;
    let index = read_index(&mut archive)?;
    let game_version = index
        .pointer("/dependencies/minecraft")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let entry = create_instance(instances_dir, name, &game_version, template)?;
    let instance_dir = instances_dir.join(&entry.folder);
    extract_prefixed(&mut archive, &instance_dir, "overrides/")?;
    extract_prefixed(&mut archive, &instance_dir, "client-overrides/")?;
    let meta_path = instance_dir.join(MODPACK_INDEX_FILE);
    fs::write(
        meta_path,
        serde_json::to_string_pretty(&index).map_err(CoreError::LoaderJson)?,
    )?;
    Ok(entry)
}

/// Installs the loader declared in `dependencies` and updates the instance version id.
pub async fn apply_mrpack_loader(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
) -> Result<InstanceEntry> {
    let index_path = instances_dir.join(&entry.folder).join(MODPACK_INDEX_FILE);
    if !index_path.is_file() {
        return Ok(entry.clone());
    }
    let body = fs::read_to_string(&index_path)?;
    let index: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    let Some(deps) = index.get("dependencies").and_then(Value::as_object) else {
        return Ok(entry.clone());
    };
    let game_version = deps
        .get("minecraft")
        .and_then(Value::as_str)
        .unwrap_or(entry.version_id.as_str());

    let loader_id = if let Some(version) = deps.get("fabric-loader").and_then(Value::as_str) {
        Some(
            install_fabric_like(
                client,
                cache,
                fabric::FABRIC_META,
                "fabric-loader",
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("quilt-loader").and_then(Value::as_str) {
        Some(
            install_fabric_like(
                client,
                cache,
                fabric::QUILT_META,
                "quilt-loader",
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("forge").and_then(Value::as_str) {
        Some(
            install_forge_like(
                client,
                cache,
                forge::ForgeFamily::Forge,
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("neoforge").and_then(Value::as_str) {
        Some(
            install_forge_like(
                client,
                cache,
                forge::ForgeFamily::NeoForge,
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else {
        None
    };

    let Some(loader_id) = loader_id else {
        return Ok(entry.clone());
    };
    update_instance_version(instances_dir, &entry.folder, &loader_id)
}

/// Downloads every client-required file from [`MODPACK_INDEX_FILE`] into the instance.
///
/// No-op when the index is missing (vanilla / CurseForge without mrpack files).
pub async fn install_mrpack_files(
    client: &Client,
    game: &SharedCache,
    settings: &SharedCache,
    instance_dir: &Path,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let index_path = instance_dir.join(MODPACK_INDEX_FILE);
    if !index_path.is_file() {
        return Ok(());
    }
    let body = fs::read_to_string(&index_path)?;
    let index: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    let tasks = mrpack_download_tasks(&index, instance_dir)?;
    let mut pending = Vec::new();
    for task in tasks {
        if file_matches_sha1(&task.destination, &task.sha1)? {
            continue;
        }
        if let Some(parent) = task.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        pending.push(task);
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
    download_all(
        client,
        &pending,
        shared_transfer_limits(),
        download_concurrency(game, settings)?,
        write_concurrency(game, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await
}

pub fn mrpack_download_tasks(index: &Value, instance_dir: &Path) -> Result<Vec<DownloadTask>> {
    let Some(files) = index.get("files").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut tasks = Vec::new();
    for file in files {
        if client_unsupported(file) {
            continue;
        }
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or(CoreError::ModpackArchive)?;
        let destination = safe_instance_path(instance_dir, path)?;
        let sha1 = file
            .pointer("/hashes/sha1")
            .and_then(Value::as_str)
            .ok_or(CoreError::ModpackArchive)?
            .to_string();
        let url = file
            .get("downloads")
            .and_then(Value::as_array)
            .and_then(|urls| urls.first())
            .and_then(Value::as_str)
            .ok_or(CoreError::ModpackArchive)?
            .to_string();
        let size = file
            .get("fileSize")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        tasks.push(DownloadTask {
            url,
            destination,
            sha1,
            size,
        });
    }
    Ok(tasks)
}

fn client_unsupported(file: &Value) -> bool {
    file.pointer("/env/client")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "unsupported")
}

fn safe_instance_path(instance_dir: &Path, relative: &str) -> Result<PathBuf> {
    let normalized = relative.replace('\\', "/");
    if normalized.is_empty()
        || Path::new(&normalized).is_absolute()
        || normalized.split('/').any(|part| part == "..")
    {
        return Err(CoreError::ModpackArchive);
    }
    Ok(instance_dir.join(normalized))
}

fn read_index(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Value> {
    let mut file = archive
        .by_name("modrinth.index.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let mut body = String::new();
    file.read_to_string(&mut body)
        .map_err(|_| CoreError::ModpackArchive)?;
    serde_json::from_str(&body).map_err(CoreError::LoaderJson)
}

fn extract_prefixed(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    instance_dir: &Path,
    prefix: &str,
) -> Result<()> {
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let name = file.name().to_string();
        let Some(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        let destination = safe_instance_path(instance_dir, relative)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}

fn file_matches_sha1(path: &Path, expected_sha1: &str) -> Result<bool> {
    use sha1::{Digest, Sha1};
    use std::io::Read as _;

    if expected_sha1.is_empty() {
        return Ok(path.is_file());
    }
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut actual = String::with_capacity(digest.len() * 2);
    for byte in digest {
        actual.push_str(&format!("{byte:02x}"));
    }
    Ok(actual.eq_ignore_ascii_case(expected_sha1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::GlobalLaunch;
    use std::io::{Cursor, Write};
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    #[test]
    fn imports_overrides_and_minecraft_version() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("modrinth.index.json", options).unwrap();
            zip.write_all(br#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#)
                .unwrap();
            zip.start_file("overrides/options.txt", options).unwrap();
            zip.write_all(b"fov:90").unwrap();
            zip.start_file("client-overrides/client.txt", options)
                .unwrap();
            zip.write_all(b"ok").unwrap();
            zip.finish().unwrap();
        }
        let bytes = buffer.into_inner();
        let root = std::env::temp_dir().join(format!("licht-mrpack-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let instances = root.join("instances");
        fs::create_dir_all(&instances).unwrap();
        let template = GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: Vec::new(),
            fullscreen: false,
            width: None,
            height: None,
        };
        let entry = import_mrpack_bytes(&instances, "Cobble", &bytes, &template).expect("import");
        assert_eq!(entry.version_id, "1.20.1");
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("options.txt")).unwrap(),
            "fov:90"
        );
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("client.txt")).unwrap(),
            "ok"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn builds_download_tasks_and_skips_unsupported_client_files() {
        let index: Value = serde_json::from_str(
            r#"{
              "files": [
                {
                  "path": "mods/a.jar",
                  "hashes": { "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
                  "downloads": ["https://example.invalid/a.jar"],
                  "fileSize": 10,
                  "env": { "client": "required", "server": "unsupported" }
                },
                {
                  "path": "mods/server-only.jar",
                  "hashes": { "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" },
                  "downloads": ["https://example.invalid/b.jar"],
                  "fileSize": 20,
                  "env": { "client": "unsupported", "server": "required" }
                }
              ]
            }"#,
        )
        .unwrap();
        let root = PathBuf::from("/tmp/instance");
        let tasks = mrpack_download_tasks(&index, &root).expect("tasks");
        assert_eq!(tasks.len(), 1);
        assert!(
            tasks[0]
                .destination
                .to_string_lossy()
                .replace('\\', "/")
                .ends_with("mods/a.jar")
        );
        assert_eq!(tasks[0].sha1, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    }

    #[test]
    fn rejects_path_escape_in_modpack_files() {
        let index: Value = serde_json::from_str(
            r#"{
              "files": [{
                "path": "../evil.jar",
                "hashes": { "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
                "downloads": ["https://example.invalid/evil.jar"],
                "fileSize": 1
              }]
            }"#,
        )
        .unwrap();
        let error = mrpack_download_tasks(&index, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
    }
}
