mod index;
mod manifest;
mod overrides;
mod plan;
#[cfg(test)]
mod test_support;

use std::fs;
use std::path::Path;

use reqwest::Client;

use self::index::{legacy_files, pending_task, stored_files};
use self::manifest::{Manifest, manifest_from_zip};
use self::overrides::{extract_override_files, list_override_files, override_prefixes};
use self::plan::{Plan, format_blocked, resolve_plan};
use super::cdn::cdn_client;
use super::storage::StoredCurseforgeFile;
use super::{CURSEFORGE_INDEX_FILE, curseforge_api_key};
use crate::instance::InstanceEntry;
use crate::modpacks::MODPACK_INDEX_FILE;
use crate::modpacks::file_hashes::ensure_md5;
use crate::modpacks::path_safety::instance_key;
use crate::transfer::verification::VerificationProgress;
use crate::{CoreError, Result, SharedCache};

/// Names that must not be overwritten at the instance root by pack content.
const PROTECTED_ROOT_NAMES: [&str; 4] = [
    "instance.json",
    ".instance.json.tmp",
    MODPACK_INDEX_FILE,
    CURSEFORGE_INDEX_FILE,
];

/// Downloads every file from the stored CurseForge index into the instance.
///
/// No-op when no files are stored. A legacy [`CURSEFORGE_INDEX_FILE`] is
/// imported into SQLite first and then removed, unless the instance has a
/// Modrinth index. Each request and redirect must be HTTPS on a CurseForge CDN
/// host. The API key is a default header on that client, not part of the
/// stored URL.
pub async fn install_curseforge_files(
    database: &crate::DatabaseClient,
    game: &SharedCache,
    settings: &crate::Database,
    instance_dir: &Path,
    progress: impl Into<crate::InstallationContext>,
) -> Result<()> {
    let progress = progress.into().component(crate::DownloadComponent::Modpack);
    let (instances, folder) = instance_key(instance_dir)?;
    let mut files = database.curseforge_files(instances, folder).await?;
    if files.is_empty() && database.modpack_header(instances, folder).await?.is_none() {
        files = import_legacy_index(database, instances, folder, instance_dir).await?;
    }
    if files.is_empty() {
        return Ok(());
    }
    progress.stage(crate::DownloadPhase::Verifying).await?;
    let mut verification = VerificationProgress::start(&progress, files.len()).await?;
    let mut pending = Vec::new();
    for file in &files {
        if let Some(item) = pending_task(file, instance_dir)? {
            pending.push(item);
        }
        verification.checked().await?;
    }
    if pending.is_empty() {
        return Ok(());
    }
    let key = curseforge_api_key()?;
    let client = cdn_client(&key)?;
    let tasks: Vec<_> = pending
        .iter()
        .map(|item| crate::VerifiedDownloadTask {
            url: item.task.url.clone(),
            destination: item.task.destination.clone(),
            size: item.task.size,
            checksum: if item.task.sha1.is_empty() {
                item.md5
                    .clone()
                    .map(crate::DownloadChecksum::Md5)
                    .unwrap_or(crate::DownloadChecksum::Unverified)
            } else {
                crate::DownloadChecksum::Sha1(item.task.sha1.clone())
            },
        })
        .collect();
    progress
        .verified_queue(
            &client,
            &tasks,
            crate::settings::download_options(game, settings).await?,
        )
        .await?;
    let mut verification = VerificationProgress::start(&progress, pending.len()).await?;
    for item in &pending {
        if let Some(expected) = &item.md5 {
            ensure_md5(&item.task.destination, expected)?;
        }
        verification.checked().await?;
    }
    Ok(())
}

/// Moves a v0.2.x `licht-curseforge.json` into SQLite. An invalid file is kept.
///
/// The rows only stay if the file is removed. Otherwise a later empty file list
/// (a pack with no files) would make the stale file look unimported again.
async fn import_legacy_index(
    database: &crate::DatabaseClient,
    instances: &Path,
    folder: &str,
    instance_dir: &Path,
) -> Result<Vec<StoredCurseforgeFile>> {
    let path = instance_dir.join(CURSEFORGE_INDEX_FILE);
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let files =
        legacy_files(&body).map_err(|error| CoreError::CurseforgeLegacyIndex(Box::new(error)))?;
    database
        .replace_curseforge_files(instances, folder, files)
        .await?;
    if let Err(error) = crate::modpacks::remove_legacy_index(&path) {
        // The job fails either way; a failed undo leaves the rows, which still win over the file.
        if let Err(undo) = database
            .replace_curseforge_files(instances, folder, Vec::new())
            .await
        {
            tracing::warn!(error = %undo, "could not undo a CurseForge index import");
        }
        return Err(error.into());
    }
    database.curseforge_files(instances, folder).await
}

pub(crate) async fn import_curseforge_into(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    bytes: &[u8],
    context: &crate::InstallationContext,
) -> Result<InstanceEntry> {
    let manifest = manifest_from_zip(bytes)?;
    let plan = if manifest.files.is_empty() {
        Plan::default()
    } else {
        context.stage(crate::DownloadPhase::Preparing).await?;
        context
            .metadata(resolve_plan(
                client,
                &curseforge_api_key()?,
                &manifest.files,
            ))
            .await?
    };
    context.stage(crate::DownloadPhase::Extracting).await?;
    let entry = import_planned(database, instances_dir, entry, bytes, &manifest, &plan).await?;
    let version = if let Some((platform, version)) = &manifest.loader {
        crate::loaders::prepare_platform_profile(
            client,
            cache,
            *platform,
            &manifest.game_version,
            crate::LoaderChannel::Manual,
            Some(version),
            context,
        )
        .await?
    } else {
        manifest.game_version
    };
    crate::update_instance_version(database, instances_dir, &entry.folder, &version).await
}

async fn import_planned(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    entry: &InstanceEntry,
    bytes: &[u8],
    manifest: &Manifest,
    plan: &Plan,
) -> Result<InstanceEntry> {
    if !plan.blocked.is_empty() {
        return Err(CoreError::CurseforgeDistributionDenied {
            mods: format_blocked(&plan.blocked),
        });
    }
    let prefixes = override_prefixes(&manifest.overrides_dir);
    let entries = list_override_files(bytes, &prefixes)?;
    let instance_dir = instances_dir.join(&entry.folder);
    extract_override_files(bytes, &instance_dir, &entries)?;
    database
        .replace_curseforge_files(instances_dir, &entry.folder, stored_files(plan)?)
        .await?;
    crate::update_instance_version(
        database,
        instances_dir,
        &entry.folder,
        &manifest.game_version,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::plan::{BlockedMod, PlannedFile};
    use super::test_support::{temp_dir, zip_with};
    use super::*;
    use crate::{GlobalLaunch, InstanceSource, ModpackSource, create_pending_instance};
    use tokio::sync::mpsc;

    const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

    #[tokio::test]
    async fn imports_overrides_and_records_the_resolved_files() {
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
        let service = crate::DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let pending = pending(&service.client(), &instances).await;
        let entry = import_planned(
            &service.client(),
            &instances,
            &pending,
            &bytes,
            &sample_manifest(),
            &plan,
        )
        .await
        .expect("import");
        assert_eq!(entry.folder, pending.folder);
        let dir = instances.join(&entry.folder);
        assert_eq!(entry.version_id.as_deref(), Some("1.20.1"));
        assert_eq!(
            fs::read_to_string(dir.join("options.txt")).unwrap(),
            "fov:90"
        );
        assert_eq!(fs::read_to_string(dir.join("legacy.txt")).unwrap(), "old");
        assert_eq!(
            fs::read_to_string(dir.join("config").join("instance.json")).unwrap(),
            "nested"
        );
        let profile = crate::load_instance(&service.client(), &dir).await.unwrap();
        assert_eq!(profile.name, pending.name);
        assert!(!dir.join("instance.json").exists());
        assert!(!dir.join(CURSEFORGE_INDEX_FILE).exists());
        assert_eq!(
            service
                .client()
                .curseforge_files(&instances, &entry.folder)
                .await
                .expect("stored files"),
            [StoredCurseforgeFile {
                relative_path: "mods/sodium.jar".into(),
                url: "https://edge.forgecdn.net/files/1/2/sodium.jar".into(),
                sha1: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
                md5: None,
                size: 10,
                project_id: 7,
                file_id: 8,
                file_name: "sodium.jar".into(),
                name: "Sodium".into(),
                required: true,
            }]
        );
        service.shutdown().await.expect("shutdown");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn preserves_the_pending_instance_when_distribution_is_denied() {
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
        let service = crate::DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let pending = pending(&service.client(), &instances).await;
        let profile = crate::load_instance(&service.client(), &instances.join(&pending.folder))
            .await
            .unwrap();
        let error = import_planned(
            &service.client(),
            &instances,
            &pending,
            &bytes,
            &sample_manifest(),
            &plan,
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "curseforge mods disallow third-party distribution: Iris (iris.jar), Sodium (sodium.jar)"
        );
        assert_eq!(
            crate::load_instance(&service.client(), &instances.join(&pending.folder))
                .await
                .unwrap(),
            profile
        );
        assert!(
            service
                .client()
                .curseforge_files(&instances, &pending.folder)
                .await
                .expect("stored files")
                .is_empty()
        );
        service.shutdown().await.expect("shutdown");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn preserves_the_pending_instance_when_an_override_is_unsafe() {
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
        let service = crate::DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("database");
        let pending = pending(&service.client(), &instances).await;
        let profile = crate::load_instance(&service.client(), &instances.join(&pending.folder))
            .await
            .unwrap();
        let error = import_planned(
            &service.client(),
            &instances,
            &pending,
            &bytes,
            &sample_manifest(),
            &Plan::default(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive), "{error}");
        assert_eq!(
            crate::load_instance(&service.client(), &instances.join(&pending.folder))
                .await
                .unwrap(),
            profile
        );
        assert!(
            !instances
                .join(&pending.folder)
                .join(CURSEFORGE_INDEX_FILE)
                .exists()
        );
        service.shutdown().await.expect("shutdown");
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
        let service =
            crate::DatabaseService::start(crate::Database::open(&cache).await.expect("database"))
                .await
                .expect("database");
        service
            .client()
            .replace_curseforge_files(
                &root,
                "instance",
                vec![StoredCurseforgeFile {
                    relative_path: "mods/a.jar".into(),
                    url: "https://edge.forgecdn.net/files/1/2/a.jar".into(),
                    sha1: Some(SHA1.into()),
                    md5: Some(MD5.into()),
                    size: 11,
                    project_id: 1,
                    file_id: 2,
                    file_name: "a.jar".into(),
                    name: "A".into(),
                    required: true,
                }],
            )
            .await
            .expect("stored files");
        let installed = install_cached(&service.client(), &cache, &instance).await;
        service.shutdown().await.expect("shutdown");
        installed.expect("cached file should not be downloaded");
        assert_eq!(fs::read(mods.join("a.jar")).unwrap(), b"hello world");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn imports_a_legacy_index_into_sqlite_and_removes_the_file() {
        let root = temp_dir("legacy");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        for path in ["mods/a.jar", "resourcepacks/b.zip"] {
            let file = instance.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"hello world").unwrap();
        }
        fs::write(instance.join(CURSEFORGE_INDEX_FILE), LEGACY_INDEX).unwrap();
        let service =
            crate::DatabaseService::start(crate::Database::open(&cache).await.expect("database"))
                .await
                .expect("database");
        let client = service.client();
        // The second run reads SQLite only, so it must not need the removed file.
        for _ in 0..2 {
            install_cached(&client, &cache, &instance)
                .await
                .expect("cached files should not be downloaded");
            assert!(!instance.join(CURSEFORGE_INDEX_FILE).exists());
        }
        let stored = client
            .curseforge_files(&root, "instance")
            .await
            .expect("stored files");
        service.shutdown().await.expect("shutdown");
        assert_eq!(
            stored
                .iter()
                .map(|file| (
                    file.relative_path.as_str(),
                    file.sha1.as_deref(),
                    file.md5.as_deref(),
                    file.required
                ))
                .collect::<Vec<_>>(),
            [
                ("mods/a.jar", Some(SHA1), Some(MD5), true),
                ("resourcepacks/b.zip", None, Some(MD5), false),
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn keeps_an_invalid_legacy_index_and_stores_nothing() {
        let root = temp_dir("legacy-invalid");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        fs::create_dir_all(&instance).unwrap();
        let unsafe_index = LEGACY_INDEX.replace("mods/a.jar", "../a.jar");
        for body in ["not json", unsafe_index.as_str()] {
            fs::write(instance.join(CURSEFORGE_INDEX_FILE), body).unwrap();
            let service = crate::DatabaseService::start(
                crate::Database::open(&cache).await.expect("database"),
            )
            .await
            .expect("database");
            let installed = install_cached(&service.client(), &cache, &instance).await;
            let stored = service
                .client()
                .curseforge_files(&root, "instance")
                .await
                .expect("stored files");
            service.shutdown().await.expect("shutdown");
            assert!(
                matches!(installed, Err(CoreError::CurseforgeLegacyIndex(_))),
                "{body}"
            );
            assert!(stored.is_empty(), "{body}");
            assert_eq!(
                fs::read_to_string(instance.join(CURSEFORGE_INDEX_FILE)).unwrap(),
                body
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ignores_a_legacy_index_in_a_modrinth_instance() {
        let root = temp_dir("legacy-modrinth");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        fs::create_dir_all(&instance).unwrap();
        fs::write(instance.join(CURSEFORGE_INDEX_FILE), LEGACY_INDEX).unwrap();
        let service =
            crate::DatabaseService::start(crate::Database::open(&cache).await.expect("database"))
                .await
                .expect("database");
        let client = service.client();
        client
            .replace_modpack(&root, "instance", modrinth_index())
            .await
            .expect("Modrinth index");
        let installed = install_cached(&client, &cache, &instance).await;
        let stored = client
            .curseforge_files(&root, "instance")
            .await
            .expect("stored files");
        service.shutdown().await.expect("shutdown");
        installed.expect("a Modrinth instance has no CurseForge files");
        assert!(stored.is_empty());
        assert!(instance.join(CURSEFORGE_INDEX_FILE).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn stored_files_take_precedence_over_a_leftover_legacy_index() {
        let root = temp_dir("legacy-leftover");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        let mods = instance.join("mods");
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("a.jar"), b"hello world").unwrap();
        fs::write(instance.join(CURSEFORGE_INDEX_FILE), b"not json").unwrap();
        let service =
            crate::DatabaseService::start(crate::Database::open(&cache).await.expect("database"))
                .await
                .expect("database");
        let client = service.client();
        let files = vec![StoredCurseforgeFile {
            relative_path: "mods/a.jar".into(),
            url: "https://edge.forgecdn.net/files/1/2/a.jar".into(),
            sha1: Some(SHA1.into()),
            md5: None,
            size: 11,
            project_id: 1,
            file_id: 2,
            file_name: "a.jar".into(),
            name: "A".into(),
            required: true,
        }];
        client
            .replace_curseforge_files(&root, "instance", files.clone())
            .await
            .expect("stored files");
        let installed = install_cached(&client, &cache, &instance).await;
        let stored = client
            .curseforge_files(&root, "instance")
            .await
            .expect("stored files");
        service.shutdown().await.expect("shutdown");
        installed.expect("the leftover file must not be parsed");
        assert_eq!(stored, files);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn keeps_the_legacy_index_and_stores_nothing_when_it_cannot_be_removed() {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 1;
        let root = temp_dir("legacy-locked");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        for path in ["mods/a.jar", "resourcepacks/b.zip"] {
            let file = instance.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"hello world").unwrap();
        }
        let index = instance.join(CURSEFORGE_INDEX_FILE);
        fs::write(&index, LEGACY_INDEX).unwrap();
        let service =
            crate::DatabaseService::start(crate::Database::open(&cache).await.expect("database"))
                .await
                .expect("database");
        let client = service.client();
        // Readers may open the index, but nobody may delete it while this handle is open.
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&index)
            .expect("locked index");
        let blocked = install_cached(&client, &cache, &instance).await;
        let stored_while_locked = client
            .curseforge_files(&root, "instance")
            .await
            .expect("stored files");
        drop(locked);
        let retried = install_cached(&client, &cache, &instance).await;
        let stored_after_retry = client
            .curseforge_files(&root, "instance")
            .await
            .expect("stored files");
        service.shutdown().await.expect("shutdown");
        assert!(matches!(blocked, Err(CoreError::Io(_))), "{blocked:?}");
        assert!(stored_while_locked.is_empty());
        retried.expect("the retry imports the index");
        assert_eq!(stored_after_retry.len(), 2);
        assert!(!index.exists());
        let _ = fs::remove_dir_all(&root);
    }

    fn modrinth_index() -> crate::modpacks::mrpack::storage::StoredModpack {
        crate::modpacks::mrpack::storage::StoredModpack {
            header: crate::modpacks::mrpack::storage::StoredModpackHeader {
                format_version: Some(1),
                game: Some("minecraft".into()),
                version_id: Some("v1".into()),
                name: Some("Modrinth".into()),
                summary: None,
            },
            dependencies: vec![("minecraft".into(), "1.20.1".into())],
            files: Vec::new(),
        }
    }

    const LEGACY_INDEX: &str =
        include_str!("../../../../tests/fixtures/curseforge/licht-curseforge.json");

    async fn install_cached(
        database: &crate::DatabaseClient,
        cache: &SharedCache,
        instance: &Path,
    ) -> Result<()> {
        let (progress, _incoming) = mpsc::channel(16);
        let settings = crate::Database::open(cache).await.expect("database");
        let installed =
            install_curseforge_files(database, cache, &settings, instance, progress).await;
        settings.close().await;
        installed
    }

    fn sample_manifest() -> Manifest {
        Manifest {
            game_version: "1.20.1".into(),
            files: Vec::new(),
            overrides_dir: "overrides".into(),
            loader: None,
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

    async fn pending(database: &crate::DatabaseClient, instances: &Path) -> InstanceEntry {
        create_pending_instance(
            database,
            instances,
            "Cobble",
            InstanceSource::Modpack {
                source: ModpackSource::Curseforge,
                project_id: "1".into(),
                version_id: "2".into(),
                icon_url: None,
            },
            &launch_template(),
        )
        .await
        .expect("pending instance")
    }
}
