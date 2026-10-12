use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use zip::ZipArchive;

use super::file_hashes::{ensure_sha512, prepare_download};
use super::path_safety::safe_instance_path;
use super::secure_download::{allowlisted_download_client, download_url_allowed};

use crate::download::{DEFAULT_RETRY, DownloadProgress, DownloadTask, download_all};
use crate::instance::{InstanceEntry, create_instance, update_instance_version};
use crate::loaders::LoaderChannel;
use crate::loaders::fabric::{self, install_fabric_like};
use crate::loaders::forge::{self, install_forge_like};
use crate::settings::{GlobalLaunch, download_concurrency, write_concurrency};
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};

pub const MODPACK_INDEX_FILE: &str = "licht-modpack.json";

/// Files at the instance root that a pack must never overwrite.
const PROTECTED_ROOT_NAMES: [&str; 2] = ["instance.json", MODPACK_INDEX_FILE];

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
/// A redirect is followed only when that hop is HTTPS and the host is
/// `cdn.modrinth.com`.
pub async fn install_mrpack_files(
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
    let files = index_files(&index, instance_dir)?;
    let mut pending = Vec::new();
    for file in &files {
        if !prepare_download(&file.task.destination, &file.task.sha1, &file.sha512)? {
            continue;
        }
        if let Some(parent) = file.task.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        pending.push(file.task.clone());
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
    // The caller's client would follow any redirect. This one checks every hop.
    let client = mrpack_download_client()?;
    download_all(
        &client,
        &pending,
        shared_transfer_limits(),
        download_concurrency(game, settings)?,
        write_concurrency(game, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await?;
    for file in &files {
        ensure_sha512(&file.task.destination, &file.sha512)?;
    }
    Ok(())
}

pub fn mrpack_download_tasks(index: &Value, instance_dir: &Path) -> Result<Vec<DownloadTask>> {
    Ok(index_files(index, instance_dir)?
        .into_iter()
        .map(|file| file.task)
        .collect())
}

/// Hosts accepted in a `.mrpack` `downloads[]` entry and on every redirect.
///
/// Modrinth's spec also lists GitHub and GitLab. Those are refused: pack files
/// installed here come from the Modrinth CDN.
/// CurseForge does not use this list. `download_curseforge_pack` fetches the
/// pack archive with the caller's client, and that import never writes
/// [`MODPACK_INDEX_FILE`], so CurseForge CDN hosts are not added. A
/// `forgecdn.net` URL inside an `.mrpack` is refused.
const MRPACK_DOWNLOAD_HOSTS: &[&str] = &["cdn.modrinth.com"];

struct PackFile {
    task: DownloadTask,
    sha512: String,
}

fn index_files(index: &Value, instance_dir: &Path) -> Result<Vec<PackFile>> {
    let Some(files) = index.get("files").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut packed = Vec::new();
    for file in files {
        if client_unsupported(file) {
            continue;
        }
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or(CoreError::ModpackArchive)?;
        let destination = safe_instance_path(instance_dir, path, &PROTECTED_ROOT_NAMES)?;
        let sha1 = required_sha1(file)?;
        let sha512 = required_sha512(file)?;
        let url = select_download_url(file)?;
        let size = file.get("fileSize").and_then(Value::as_u64).unwrap_or(0);
        packed.push(PackFile {
            task: DownloadTask {
                url,
                destination,
                sha1,
                size,
            },
            sha512,
        });
    }
    Ok(packed)
}

fn required_sha1(file: &Value) -> Result<String> {
    file.pointer("/hashes/sha1")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(CoreError::ModpackArchive)
}

fn required_sha512(file: &Value) -> Result<String> {
    file.pointer("/hashes/sha512")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(CoreError::ModpackArchive)
}

fn mrpack_download_client() -> Result<Client> {
    allowlisted_download_client(
        MRPACK_DOWNLOAD_HOSTS,
        "modpack redirect target is not allowed",
        "too many modpack redirects",
    )
}

fn select_download_url(file: &Value) -> Result<String> {
    let urls = file
        .get("downloads")
        .and_then(Value::as_array)
        .ok_or(CoreError::ModpackArchive)?;
    for entry in urls {
        let raw = entry.as_str().ok_or(CoreError::ModpackArchive)?;
        if download_url_allowed(raw, MRPACK_DOWNLOAD_HOSTS) {
            return Ok(raw.to_string());
        }
    }
    Err(CoreError::ModpackArchive)
}

fn client_unsupported(file: &Value) -> bool {
    file.pointer("/env/client")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "unsupported")
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
        let name = file.name().replace('\\', "/");
        let Some(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        // Rejects absolute paths and `..` that leave the archive. `C:foo` still
        // looks like a normal name on Unix, so `safe_instance_path` checks it too.
        if file.enclosed_name().is_none() {
            return Err(CoreError::ModpackArchive);
        }
        let destination = safe_instance_path(instance_dir, relative, &PROTECTED_ROOT_NAMES)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::GlobalLaunch;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
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
                  "hashes": {
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "sha512": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                  },
                  "downloads": ["https://cdn.modrinth.com/data/a.jar"],
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
        assert_eq!(tasks[0].url, "https://cdn.modrinth.com/data/a.jar");
    }

    #[test]
    fn rejects_path_escape_in_modpack_files() {
        let index: Value = serde_json::from_str(
            r#"{
              "files": [{
                "path": "../evil.jar",
                "hashes": { "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
                "downloads": ["https://cdn.modrinth.com/evil.jar"],
                "fileSize": 1
              }]
            }"#,
        )
        .unwrap();
        let error = mrpack_download_tasks(&index, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
    }

    #[test]
    fn accepts_modrinth_download_hosts_and_skips_others() {
        let index = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": {
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "sha512": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                },
                "downloads": [
                    "http://cdn.modrinth.com/a.jar",
                    "https://evil.example/a.jar",
                    "https://user:secret@cdn.modrinth.com/a.jar",
                    "https://cdn.modrinth.com:444/a.jar",
                    "https://github.com/user/repo/releases/download/v1/a.jar",
                    "https://raw.githubusercontent.com/user/repo/main/a.jar",
                    "https://gitlab.com/user/repo/-/raw/main/a.jar",
                    "https://edge.forgecdn.net/files/1/2/a.jar",
                    "https://cdn.modrinth.com/data/a.jar"
                ],
                "fileSize": 1
            }]
        });
        let tasks = mrpack_download_tasks(&index, Path::new("/tmp/instance")).expect("tasks");
        assert_eq!(tasks[0].url, "https://cdn.modrinth.com/data/a.jar");
    }

    #[test]
    fn rejects_packs_with_no_allowed_download_url() {
        for url in [
            "http://cdn.modrinth.com/a.jar",
            "https://example.invalid/a.jar",
            "https://cdn.modrinth.com.evil.com/a.jar",
            "https://evil.cdn.modrinth.com/a.jar",
            "https://1.2.3.4/a.jar",
            "https://github.com/user/repo/releases/download/v1/a.jar",
            "https://raw.githubusercontent.com/user/repo/main/a.jar",
            "https://gitlab.com/user/repo/-/raw/main/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar",
            "https://mediafilez.forgecdn.net/files/1/2/a.jar",
        ] {
            let index = file_index("mods/a.jar", url);
            let error = mrpack_download_tasks(&index, Path::new("/tmp/instance")).unwrap_err();
            assert!(matches!(error, CoreError::ModpackArchive), "{url}");
        }
    }

    #[test]
    fn allows_the_modrinth_cdn() {
        for url in [
            "https://cdn.modrinth.com/data/a.jar",
            "https://CDN.MODRINTH.COM/data/a.jar",
        ] {
            let tasks =
                mrpack_download_tasks(&file_index("mods/a.jar", url), Path::new("/tmp/instance"))
                    .unwrap_or_else(|_| panic!("{url}"));
            assert_eq!(tasks[0].url, url);
        }
    }

    #[test]
    fn requires_sha1_and_sha512() {
        let missing_sha1 = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": { "sha512": "ab" },
                "downloads": ["https://cdn.modrinth.com/a.jar"],
                "fileSize": 1
            }]
        });
        let empty_sha1 = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": { "sha1": "  ", "sha512": "ab" },
                "downloads": ["https://cdn.modrinth.com/a.jar"],
                "fileSize": 1
            }]
        });
        let missing_sha512 = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": { "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
                "downloads": ["https://cdn.modrinth.com/a.jar"],
                "fileSize": 1
            }]
        });
        let empty_sha512 = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": {
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "sha512": "  "
                },
                "downloads": ["https://cdn.modrinth.com/a.jar"],
                "fileSize": 1
            }]
        });
        let non_string_sha512 = serde_json::json!({
            "files": [{
                "path": "mods/a.jar",
                "hashes": {
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "sha512": 1
                },
                "downloads": ["https://cdn.modrinth.com/a.jar"],
                "fileSize": 1
            }]
        });
        for index in [
            missing_sha1,
            empty_sha1,
            missing_sha512,
            empty_sha512,
            non_string_sha512,
        ] {
            let error = mrpack_download_tasks(&index, Path::new("/tmp/instance")).unwrap_err();
            assert!(matches!(error, CoreError::ModpackArchive));
        }
    }

    #[test]
    fn rejects_launcher_files_reserved_names_and_drive_prefixes() {
        for path in [
            "instance.json",
            "Instance.JSON",
            "licht-modpack.json",
            "C:evil.jar",
            "mods/C:evil.jar",
            "CON",
            "mods/CON.txt",
            "mods/lpt1.jar",
            "mods/foo.",
            "../evil.jar",
            "/etc/passwd",
            "mods/../../outside.jar",
        ] {
            let error = mrpack_download_tasks(
                &file_index(path, "https://cdn.modrinth.com/a.jar"),
                Path::new("/tmp/instance"),
            )
            .unwrap_err();
            assert!(matches!(error, CoreError::ModpackArchive), "{path}");
        }
        let tasks = mrpack_download_tasks(
            &file_index("config/instance.json", "https://cdn.modrinth.com/a.jar"),
            Path::new("/tmp/instance"),
        )
        .expect("a nested instance.json is a normal config file");
        assert!(
            tasks[0]
                .destination
                .to_string_lossy()
                .replace('\\', "/")
                .ends_with("config/instance.json")
        );
    }

    #[test]
    fn rejects_overrides_that_target_launcher_files_or_windows_devices() {
        let names = [
            "overrides/instance.json",
            "overrides/licht-modpack.json",
            "overrides/C:evil.txt",
            "overrides/CON",
            "overrides/mods/CON.txt",
            "client-overrides/../../outside.txt",
        ];
        for name in names {
            let bytes = zip_with(&[
                (
                    "modrinth.index.json",
                    br#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#,
                ),
                (name, b"pwned"),
            ]);
            let root = temp_dir("override");
            let instances = root.join("instances");
            fs::create_dir_all(&instances).unwrap();
            let error = import_mrpack_bytes(&instances, "Cobble", &bytes, &launch_template())
                .expect_err(name);
            assert!(matches!(error, CoreError::ModpackArchive), "{name}");
            let instance_dir = fs::read_dir(&instances)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let profile = fs::read_to_string(instance_dir.join("instance.json")).unwrap();
            assert!(!profile.contains("pwned"), "{name}");
            assert!(!instance_dir.join("C:evil.txt").exists(), "{name}");
            assert!(!instance_dir.join("CON").exists(), "{name}");
            let _ = fs::remove_dir_all(&root);
        }
    }

    fn file_index(path: &str, url: &str) -> Value {
        serde_json::json!({
            "files": [{
                "path": path,
                "hashes": {
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "sha512": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                },
                "downloads": [url],
                "fileSize": 1
            }]
        })
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
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-mrpack-{label}-{}-{stamp}",
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
            for (name, bytes) in files {
                zip.start_file(*name, options).unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }
}
