use std::path::Path;

use serde_json::Value;

use super::PROTECTED_ROOT_NAMES;
use crate::download::DownloadTask;
use crate::modpacks::path_safety::safe_instance_path;
use crate::modpacks::secure_download::download_url_allowed;
use crate::{CoreError, Result};

/// Hosts accepted in a `.mrpack` `downloads[]` entry and on every redirect.
///
/// Modrinth's spec also lists GitHub and GitLab. Those are refused: pack files
/// installed here come from the Modrinth CDN.
/// CurseForge does not use this list. `download_curseforge_pack` fetches the
/// pack archive with the caller's client, and that import never writes
/// [`super::MODPACK_INDEX_FILE`], so CurseForge CDN hosts are not added. A
/// `forgecdn.net` URL inside an `.mrpack` is refused.
pub(super) const MRPACK_DOWNLOAD_HOSTS: &[&str] = &["cdn.modrinth.com"];

pub(super) struct PackFile {
    pub task: DownloadTask,
    pub sha512: String,
}

pub fn mrpack_download_tasks(index: &Value, instance_dir: &Path) -> Result<Vec<DownloadTask>> {
    Ok(index_files(index, instance_dir)?
        .into_iter()
        .map(|file| file.task)
        .collect())
}

pub(super) fn index_files(index: &Value, instance_dir: &Path) -> Result<Vec<PackFile>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
}
