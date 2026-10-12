mod index;
mod manifest;
mod overrides;
mod plan;
#[cfg(test)]
mod test_support;

use std::fs;
use std::path::Path;

use reqwest::Client;
use tokio::sync::mpsc;

use self::index::{CurseforgeIndex, pending_task, write_index};
use self::manifest::{Manifest, manifest_from_zip};
use self::overrides::{extract_override_files, list_override_files, override_prefixes};
use self::plan::{Plan, format_blocked, resolve_plan};
use super::cdn::cdn_client;
use super::{CURSEFORGE_INDEX_FILE, curseforge_api_key};
use crate::download::{DEFAULT_RETRY, DownloadProgress, DownloadTask, download_all};
use crate::instance::{InstanceEntry, create_instance};
use crate::modpacks::MODPACK_INDEX_FILE;
use crate::modpacks::file_hashes::ensure_md5;
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

#[cfg(test)]
mod tests {
    use super::index::{IndexFile, IndexHashes};
    use super::plan::{BlockedMod, PlannedFile};
    use super::test_support::{temp_dir, zip_with};
    use super::*;
    use serde_json::Value;

    const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

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
    fn does_not_create_an_instance_when_an_override_is_unsafe() {
        let bytes = zip_with(&[
            (
                "manifest.json",
                br#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
            ),
            ("overrides/instance.json", b"pwned"),
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
        .unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive), "{error}");
        assert!(fs::read_dir(&instances).unwrap().next().is_none());
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
}
