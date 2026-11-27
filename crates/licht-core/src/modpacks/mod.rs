//! Modpack discovery (Modrinth / CurseForge) and `.mrpack` import.

mod archive;
mod archive_details;
pub(crate) mod curseforge;
mod description;
mod discovery;
mod file_hashes;
mod modrinth;
pub(crate) mod mrpack;
mod path_safety;
mod secure_download;
mod types;
pub(crate) mod workspace;

pub use curseforge::{CURSEFORGE_API_KEY_ENV, CURSEFORGE_INDEX_FILE, install_curseforge_files};
pub use discovery::{available_modpack_sources, get_modpack, search_modpacks};
pub use mrpack::{
    MODPACK_INDEX_FILE, apply_mrpack_loader, install_mrpack_files, mrpack_download_tasks,
};
pub use types::{
    ModpackCategory, ModpackDetails, ModpackListItem, ModpackLoader, ModpackSearchFilters,
    ModpackSearchPage, ModpackSort, ModpackSource, ModpackVersionItem,
};

use reqwest::Client;

use crate::instance::content::DISABLED_SUFFIX;
use crate::instance::{InstanceEntry, InstanceSource, LinkedModpack, instance_entry};
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result, SharedCache};
use std::path::Path;

/// The pack version a store download resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackVersion {
    pub name: String,
    pub published_at: Option<String>,
}

/// What to keep about a store pack: the store's name and author, falling back to what
/// the pack's own archive declares, with the version the download resolved. The
/// version is the store's name even when a manifest declares one, so it matches
/// what `refresh_linked_modpack_version` keeps it in line with.
fn linked_modpack(
    store: Option<&ModpackDetails>,
    archive: Option<LinkedModpack>,
    version: PackVersion,
) -> Option<LinkedModpack> {
    let (name, author) = match store {
        Some(details) => (
            archive_details::capped(&details.name),
            archive_details::capped(&details.author),
        ),
        None => {
            let archive = archive?;
            (Some(archive.name), archive.author)
        }
    };
    Some(LinkedModpack {
        name: name?,
        author,
        version_name: archive_details::capped(&version.name)?,
        published_at: version
            .published_at
            .as_deref()
            .and_then(archive_details::capped),
    })
}

/// The store page of the pack. It only feeds the UI, so a store that cannot describe
/// the pack does not stop the install, but cancelling or pausing still does.
async fn store_details(
    client: &Client,
    context: &crate::InstallationContext,
    source: ModpackSource,
    project_id: &str,
) -> Result<Option<ModpackDetails>> {
    match context
        .metadata(get_modpack(client, source, project_id))
        .await
    {
        Ok(details) => Ok(Some(details)),
        Err(CoreError::DownloadCancelled) => Err(CoreError::DownloadCancelled),
        Err(error) => {
            tracing::warn!(%error, "could not read the modpack's store details");
            Ok(None)
        }
    }
}

/// Removes a v0.2.x index file after its rows were stored. A file that is
/// already gone was removed by another run that stored the same rows, so the
/// import stands.
pub(crate) fn remove_legacy_index(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        removed => removed,
    }
}

/// Every version of a pack, newest first (undated versions last).
pub async fn list_modpack_versions(
    client: &Client,
    source: ModpackSource,
    project_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    let mut versions = match source {
        ModpackSource::Modrinth => modrinth::list_modrinth_versions(client, project_id).await?,
        ModpackSource::Curseforge => {
            curseforge::list_curseforge_versions(client, project_id).await?
        }
    };
    // RFC 3339 UTC timestamps compare correctly as text; `None` sorts last here.
    versions.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    Ok(versions)
}

/// Keeps a store pack's saved version name in line with the store: instances
/// installed before version names were tidied saved the long form ("Homestead
/// 2.0.1"), and authors can edit a version's name later. `None` when the
/// instance is not a store pack or nothing changed.
pub async fn refresh_linked_modpack_version(
    database: &crate::DatabaseClient,
    client: &Client,
    instances_dir: &Path,
    folder: &str,
) -> Result<Option<InstanceEntry>> {
    let Some(linked) =
        crate::instance::linked_store_version(database, instances_dir, folder).await?
    else {
        return Ok(None);
    };
    let versions = list_modpack_versions(client, linked.source, &linked.project_id).await?;
    let Some(renamed) = versions
        .iter()
        .find(|version| version.id == linked.version_id)
        .and_then(|version| refreshed_version_name(&linked.version_name, &version.name))
    else {
        return Ok(None);
    };
    crate::instance::rename_linked_modpack_version(
        database,
        instances_dir,
        folder,
        &linked,
        &renamed,
    )
    .await
}

/// The store's name for a version, when it differs from the saved one. The
/// saved name follows the store, so edits made there reach the instance.
fn refreshed_version_name(saved: &str, store: &str) -> Option<String> {
    let (saved, store) = (saved.trim(), store.trim());
    (!store.is_empty() && saved != store).then(|| store.to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_instance_from_modpack(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    name: &str,
    source: ModpackSource,
    project_id: &str,
    version_id: &str,
    icon_url: Option<&str>,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let selection = InstanceSource::Modpack {
        source,
        project_id: project_id.to_string(),
        version_id: version_id.to_string(),
        icon_url: icon_url.map(str::to_string),
    };
    let entry =
        crate::create_pending_instance(database, instances_dir, name, selection.clone(), template)
            .await?;
    prepare_modpack_instance(
        database,
        client,
        cache,
        instances_dir,
        &entry,
        &selection,
        &crate::InstallationContext::default(),
    )
    .await
}

pub(crate) async fn prepare_modpack_instance(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    selection: &InstanceSource,
    context: &crate::InstallationContext,
) -> Result<InstanceEntry> {
    install_modpack(
        database,
        client,
        cache,
        instances_dir,
        entry,
        selection,
        context,
        ContentReset::Keep,
    )
    .await
}

/// Installs a ready instance's store pack again. Its content folders are only
/// cleared once the new archive has downloaded, so a cancelled or failed download
/// leaves the instance as it was.
pub(crate) async fn reinstall_modpack_instance(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    selection: &InstanceSource,
    context: &crate::InstallationContext,
) -> Result<InstanceEntry> {
    if !matches!(selection, InstanceSource::Modpack { .. }) {
        return Err(CoreError::InstanceNotLinked);
    }
    install_modpack(
        database,
        client,
        cache,
        instances_dir,
        entry,
        selection,
        context,
        ContentReset::Folders,
    )
    .await
}

/// Installs another version of a ready instance's store pack. Only the previous
/// version's content files are removed, once the new archive has downloaded, so
/// content the user added stays.
pub(crate) async fn update_modpack_instance(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    selection: &InstanceSource,
    context: &crate::InstallationContext,
) -> Result<InstanceEntry> {
    if !matches!(selection, InstanceSource::Modpack { .. }) {
        return Err(CoreError::InstanceNotLinked);
    }
    install_modpack(
        database,
        client,
        cache,
        instances_dir,
        entry,
        selection,
        context,
        ContentReset::PackFiles,
    )
    .await
}

/// What an install clears from the instance before importing the pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentReset {
    /// A first install: nothing is there yet.
    Keep,
    /// A reinstall: the content folders, with anything the user added.
    Folders,
    /// A version change: the previous version's content files only.
    PackFiles,
}

const CONTENT_FOLDERS: [&str; 3] = ["mods", "resourcepacks", "shaderpacks"];

/// Clears the content folders a reinstall replaces, with anything the user added to
/// them. Worlds and other folders stay. The pack's stored rows are kept: the import
/// replaces them, and until it does, Repair can still restore the previous files.
fn reset_pack_content(dir: &Path) -> Result<()> {
    for name in CONTENT_FOLDERS {
        // `remove_dir_all` removes a linked folder's link, not the folder it points to.
        match std::fs::remove_dir_all(dir.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// The content files of the instance's pack (`mods/a.jar`) the user disabled: a
/// file the pack ships enabled that is now only there as `name.disabled`.
async fn disabled_pack_content(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    dir: &Path,
) -> Result<Vec<String>> {
    let mut disabled = Vec::new();
    for relative in pack_content_paths(database, instances_dir, folder).await? {
        if relative.ends_with(DISABLED_SUFFIX) {
            continue;
        }
        let path = path_safety::safe_instance_path(dir, &relative, &[])?;
        let toggled = path.with_file_name(format!(
            "{}{DISABLED_SUFFIX}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
        ));
        if std::fs::symlink_metadata(&path).is_err()
            && std::fs::symlink_metadata(&toggled).is_ok_and(|metadata| metadata.is_file())
        {
            disabled.push(relative);
        }
    }
    Ok(disabled)
}

/// Disables again, in the new version, the pack files the user had disabled in the
/// previous one. Files the new version no longer ships are skipped.
pub(crate) fn disable_content_again(dir: &Path, disabled: &[String]) -> Result<()> {
    for relative in disabled {
        let path = path_safety::safe_instance_path(dir, relative, &[])?;
        let toggled = path.with_file_name(format!(
            "{}{DISABLED_SUFFIX}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
        ));
        if std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file())
            && std::fs::symlink_metadata(&toggled).is_err()
        {
            std::fs::rename(&path, &toggled)?;
        }
    }
    Ok(())
}

/// The instance-relative content paths (under `mods`, `resourcepacks` or
/// `shaderpacks`) the instance's pack rows list.
async fn pack_content_paths(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<Vec<String>> {
    let paths = database
        .modpack_files(instances_dir, folder)
        .await?
        .into_iter()
        .map(|file| file.relative_path)
        .chain(
            database
                .curseforge_files(instances_dir, folder)
                .await?
                .into_iter()
                .map(|file| file.relative_path),
        )
        .chain(
            database
                .modpack_override_files(instances_dir, folder)
                .await?,
        );
    Ok(paths
        .map(|relative| relative.replace('\\', "/"))
        .filter(|relative| {
            relative
                .split_once('/')
                .is_some_and(|(top, _)| CONTENT_FOLDERS.contains(&top))
        })
        .collect())
}

/// Removes the content files (mods, resource packs, shader packs) the instance's
/// pack installed, whether the user left them enabled or disabled, and the folder
/// packs they empty. Content the user added and other pack files, such as configs
/// the next version overwrites, stay. Links are never followed.
async fn remove_pack_content_files(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    dir: &Path,
) -> Result<()> {
    for relative in pack_content_paths(database, instances_dir, folder).await? {
        let Some((top, _)) = relative.split_once('/') else {
            continue;
        };
        let path = path_safety::safe_instance_path(dir, &relative, &[])?;
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let toggled = match name.strip_suffix(DISABLED_SUFFIX) {
            Some(enabled) => enabled.to_string(),
            None => format!("{name}{DISABLED_SUFFIX}"),
        };
        for candidate in [path.clone(), path.with_file_name(toggled)] {
            if std::fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                std::fs::remove_file(&candidate)?;
            }
        }
        remove_empty_parents(dir.join(top), &path)?;
    }
    Ok(())
}

/// Removes the folders between `root` and `path` that removing `path` left empty.
fn remove_empty_parents(root: std::path::PathBuf, path: &Path) -> Result<()> {
    let mut current = path.parent();
    while let Some(folder) = current {
        if folder == root || !folder.starts_with(&root) {
            break;
        }
        match std::fs::remove_dir(folder) {
            Ok(()) => {}
            // Not empty, or already gone: the folders above stay too.
            Err(_) if folder.exists() => break,
            Err(_) => {}
        }
        current = folder.parent();
    }
    Ok(())
}

// One setting separates a first install, a reinstall and a version change; the
// steps are otherwise shared.
#[allow(clippy::too_many_arguments)]
async fn install_modpack(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    selection: &InstanceSource,
    context: &crate::InstallationContext,
    reset: ContentReset,
) -> Result<InstanceEntry> {
    let (source, remote) = match selection {
        InstanceSource::Modpack {
            source,
            project_id,
            version_id,
            icon_url: _,
        } => (source, Some((project_id, version_id))),
        // The archive was staged in the workspace when the instance was created.
        InstanceSource::ModpackFile { source } => (source, None),
        _ => return Err(CoreError::InstancePreparing),
    };
    selection.validate()?;
    let context = context.component(crate::DownloadComponent::ModpackArchive);
    context.stage(crate::DownloadPhase::Preparing).await?;
    let dir = instances_dir.join(&entry.folder);
    restore_instance_icon(client, &dir, selection, &context).await?;
    let archive = workspace::archive_path(&dir)?;
    let mut resolved = None;
    if let Some((project_id, version_id)) = remote {
        let version = match source {
            ModpackSource::Modrinth => {
                modrinth::download_modrinth_mrpack_to(
                    client, project_id, version_id, &archive, &context,
                )
                .await?
            }
            ModpackSource::Curseforge => {
                curseforge::download_curseforge_pack_to(
                    client, project_id, version_id, &archive, &context,
                )
                .await?
            }
        };
        let store = store_details(client, &context, *source, project_id).await?;
        resolved = Some((version, store));
    }
    context.stage(crate::DownloadPhase::Extracting).await?;
    match reset {
        ContentReset::Keep => {}
        ContentReset::Folders => reset_pack_content(&dir)?,
        ContentReset::PackFiles => {
            // Past this point the change can only finish: record that, with the
            // files the user disabled, before anything is removed.
            let disabled =
                disabled_pack_content(database, instances_dir, &entry.folder, &dir).await?;
            crate::instance::commit_modpack_content_change(
                database,
                instances_dir,
                &entry.folder,
                disabled,
            )
            .await?;
            remove_pack_content_files(database, instances_dir, &entry.folder, &dir).await?;
        }
    }
    let bytes = std::fs::read(&archive)?;
    let declared = archive_details::archive_details(*source, &bytes);
    let details = match resolved {
        Some((version, store)) => linked_modpack(store.as_ref(), declared, version),
        None => declared,
    };
    if let Some(details) = details {
        crate::instance::record_linked_modpack(database, instances_dir, &entry.folder, details)
            .await?;
    }
    let entry = match source {
        ModpackSource::Modrinth => {
            let entry = mrpack::import_mrpack_into(database, instances_dir, entry, &bytes).await?;
            mrpack::apply_mrpack_loader_with_context(
                database,
                client,
                cache,
                instances_dir,
                &entry,
                &context,
            )
            .await?
        }
        ModpackSource::Curseforge => {
            curseforge::import_curseforge_into(
                database,
                client,
                cache,
                instances_dir,
                entry,
                &bytes,
                &context,
            )
            .await?
        }
    };
    context.checkpoint().await?;
    instance_entry(database, &dir, &entry.folder).await
}

/// Identifies a local modpack archive by the manifest it carries.
pub(crate) fn detect_modpack_source(archive: &Path) -> Result<ModpackSource> {
    let file = std::fs::File::open(archive)?;
    let zip = zip::ZipArchive::new(file).map_err(|_| CoreError::ModpackArchive)?;
    if zip
        .file_names()
        .any(|name| name.is_ok_and(|name| name == "modrinth.index.json"))
    {
        Ok(ModpackSource::Modrinth)
    } else if zip
        .file_names()
        .any(|name| name.is_ok_and(|name| name == "manifest.json"))
    {
        Ok(ModpackSource::Curseforge)
    } else {
        Err(CoreError::ModpackArchive)
    }
}

/// Copies a local archive into the instance's installation workspace.
pub(crate) fn stage_modpack_archive(instance: &Path, archive: &Path) -> Result<()> {
    let destination = workspace::archive_path(instance)?;
    if let Some(directory) = destination.parent() {
        std::fs::create_dir_all(directory)?;
    }
    std::fs::copy(archive, destination)?;
    Ok(())
}

pub(crate) async fn restore_instance_icon(
    client: &Client,
    dir: &Path,
    source: &InstanceSource,
    context: &crate::InstallationContext,
) -> Result<()> {
    if let InstanceSource::Modpack {
        icon_url: Some(url),
        ..
    } = source
    {
        match context
            .metadata(crate::download_instance_icon(client, dir, url))
            .await
        {
            Ok(()) => {}
            Err(CoreError::DownloadCancelled) => return Err(CoreError::DownloadCancelled),
            Err(error) => tracing::warn!(error = %error, "instance icon download failed"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use licht_testkit::temp::TempDir;

    use super::*;

    #[test]
    fn a_saved_version_name_follows_the_store() {
        let refresh = refreshed_version_name;
        assert_eq!(
            refresh("Homestead 2.0.1", "2.0.1").as_deref(),
            Some("2.0.1")
        );
        // Modrinth's old `name` need not end with the version number.
        assert_eq!(
            refresh("15.0.0-beta.4 for 26.3", "15.0.0-beta.4").as_deref(),
            Some("15.0.0-beta.4")
        );
        assert_eq!(
            refresh("Fresh & Smooth 3.0.0 ", "3.0.0+26.3").as_deref(),
            Some("3.0.0+26.3")
        );
        // An edit on the store reaches the saved name, even a manifest one.
        assert_eq!(
            refresh("2.0.1", "2.0.1 Hotfix").as_deref(),
            Some("2.0.1 Hotfix")
        );
        assert_eq!(refresh("2.0.1", "2.0.1"), None);
        assert_eq!(refresh(" 2.0.1 ", "2.0.1"), None);
        assert_eq!(refresh("Homestead 2.0.1", " "), None);
    }

    fn details(author: &str) -> ModpackDetails {
        ModpackDetails {
            id: "fo".into(),
            source: ModpackSource::Modrinth,
            name: "Fabulously Optimized".into(),
            summary: String::new(),
            author: author.into(),
            downloads: 0,
            followers: None,
            icon_url: None,
            categories: Vec::new(),
            loaders: Vec::new(),
            game_versions: Vec::new(),
            license: None,
            published_at: None,
            updated_at: None,
            gallery: Vec::new(),
            project_url: None,
            links: Default::default(),
            description_html: String::new(),
        }
    }

    fn version() -> PackVersion {
        PackVersion {
            name: "15.0.0-beta.3".into(),
            published_at: Some("2026-11-01T10:00:00Z".into()),
        }
    }

    #[test]
    fn keeps_the_store_name_author_and_installed_version() {
        assert_eq!(
            linked_modpack(Some(&details("robotkoer")), None, version()),
            Some(LinkedModpack {
                name: "Fabulously Optimized".into(),
                author: Some("robotkoer".into()),
                version_name: "15.0.0-beta.3".into(),
                published_at: Some("2026-11-01T10:00:00Z".into()),
            })
        );
        assert_eq!(
            linked_modpack(Some(&details("  ")), None, version()).and_then(|pack| pack.author),
            None
        );
    }

    #[test]
    fn falls_back_to_the_archive_name_when_the_store_cannot_describe_the_pack() {
        let declared = LinkedModpack {
            name: "Fabulously Optimized".into(),
            author: None,
            version_name: "archive 15".into(),
            published_at: None,
        };
        assert_eq!(
            linked_modpack(None, Some(declared), version()),
            Some(LinkedModpack {
                name: "Fabulously Optimized".into(),
                author: None,
                version_name: "15.0.0-beta.3".into(),
                published_at: Some("2026-11-01T10:00:00Z".into()),
            })
        );
        assert_eq!(linked_modpack(None, None, version()), None);
    }

    #[test]
    fn the_store_version_wins_over_the_manifest_version() {
        // The store's name can be edited later and the refresh follows it, so
        // installs save the same name the refresh would.
        let declared = LinkedModpack {
            name: "All the Mods 10".into(),
            author: None,
            version_name: "8.5-manifest".into(),
            published_at: None,
        };
        let file = PackVersion {
            name: "8.5".into(),
            published_at: None,
        };
        assert_eq!(
            linked_modpack(Some(&details("ATM Team")), Some(declared), file)
                .map(|pack| pack.version_name)
                .as_deref(),
            Some("8.5")
        );
    }

    #[test]
    fn caps_long_store_text() {
        let mut store = details("robotkoer");
        store.name = "x".repeat(500);
        let pack = linked_modpack(Some(&store), None, version()).expect("details");
        assert_eq!(pack.name.chars().count(), 200);
    }

    #[tokio::test]
    async fn a_reinstall_clears_only_the_content_folders_and_keeps_the_pack_rows() {
        let temp = TempDir::new("reinstall-reset");
        let root = temp.path().to_path_buf();
        let service = crate::DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("service");
        let client = service.client();
        let instances = root.join("instances");
        let dir = instances.join("Pack");
        for folder in [
            "mods",
            "resourcepacks",
            "shaderpacks/Folder Pack",
            "saves/World",
            "config",
        ] {
            std::fs::create_dir_all(dir.join(folder)).expect("folder");
        }
        std::fs::write(dir.join("mods").join("extra.jar"), b"mod").expect("mod");
        std::fs::write(dir.join("saves").join("World").join("level.dat"), b"world").expect("world");
        std::fs::write(dir.join("config").join("mod.toml"), b"config").expect("config");
        client
            .replace_modpack_override_files(&instances, "Pack", vec!["config/mod.toml".into()])
            .await
            .expect("overrides");

        reset_pack_content(&dir).expect("reset");

        for folder in ["mods", "resourcepacks", "shaderpacks"] {
            assert!(!dir.join(folder).exists(), "{folder} is cleared");
        }
        assert!(dir.join("saves").join("World").join("level.dat").is_file());
        assert!(dir.join("config").join("mod.toml").is_file());
        // Until the import replaces them, Repair can still restore the old pack.
        assert_eq!(
            client
                .modpack_override_files(&instances, "Pack")
                .await
                .expect("overrides"),
            ["config/mod.toml"]
        );
        reset_pack_content(&dir).expect("an already cleared instance resets again");
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn a_version_change_removes_only_the_old_packs_content_files() {
        let temp = TempDir::new("version-change-reset");
        let root = temp.path().to_path_buf();
        let service = crate::DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("service");
        let client = service.client();
        let instances = root.join("instances");
        let dir = instances.join("Pack");
        for folder in ["mods", "shaderpacks/Pack Shader/shaders", "config"] {
            std::fs::create_dir_all(dir.join(folder)).expect("folder");
        }
        for file in [
            "mods/pack.jar",
            "mods/toggled.jar.disabled",
            "mods/extra.jar",
            "shaderpacks/Pack Shader/shaders/final.fsh",
            "config/pack.toml",
        ] {
            std::fs::write(dir.join(file), b"file").expect("file");
        }
        client
            .replace_curseforge_files(
                &instances,
                "Pack",
                ["mods/pack.jar", "mods/toggled.jar"]
                    .into_iter()
                    .enumerate()
                    .map(|(id, path)| curseforge::storage::StoredCurseforgeFile {
                        relative_path: path.into(),
                        url: "https://edge.forgecdn.net/files/1/2/a.jar".into(),
                        sha1: Some("a".repeat(40)),
                        md5: None,
                        size: 4,
                        project_id: id as i64,
                        file_id: id as i64,
                        file_name: "a.jar".into(),
                        name: "A".into(),
                        required: true,
                    })
                    .collect(),
            )
            .await
            .expect("files");
        client
            .replace_modpack_override_files(
                &instances,
                "Pack",
                vec![
                    "shaderpacks/Pack Shader/shaders/final.fsh".into(),
                    "config/pack.toml".into(),
                ],
            )
            .await
            .expect("overrides");

        let disabled = disabled_pack_content(&client, &instances, "Pack", &dir)
            .await
            .expect("disabled");
        assert_eq!(disabled, ["mods/toggled.jar"]);
        remove_pack_content_files(&client, &instances, "Pack", &dir)
            .await
            .expect("remove");

        assert!(!dir.join("mods/pack.jar").exists());
        assert!(
            !dir.join("mods/toggled.jar.disabled").exists(),
            "toggled pack file"
        );
        assert!(
            !dir.join("shaderpacks/Pack Shader").exists(),
            "emptied folder pack"
        );
        assert!(
            dir.join("mods/extra.jar").is_file(),
            "the user's own mod stays"
        );
        assert!(
            dir.join("config/pack.toml").is_file(),
            "the new pack overwrites configs"
        );
        assert!(dir.join("shaderpacks").is_dir());
        service.shutdown().await.expect("shutdown");
    }

    #[test]
    fn the_new_version_keeps_the_pack_files_the_user_disabled_disabled() {
        let temp = TempDir::new("version-change-disabled");
        let dir = temp.path().join("Pack");
        std::fs::create_dir_all(dir.join("mods")).expect("mods");
        std::fs::write(dir.join("mods/sodium.jar"), b"new").expect("shipped again");
        std::fs::write(dir.join("mods/kept.jar"), b"new").expect("other mod");

        disable_content_again(
            &dir,
            &[
                "mods/sodium.jar".to_string(),
                "mods/dropped.jar".to_string(),
            ],
        )
        .expect("disable");

        assert!(!dir.join("mods/sodium.jar").exists());
        assert!(dir.join("mods/sodium.jar.disabled").is_file());
        assert!(dir.join("mods/kept.jar").is_file());
        assert!(!dir.join("mods/dropped.jar.disabled").exists());
    }

    #[test]
    fn a_legacy_index_already_removed_by_another_run_counts_as_removed() {
        let directory = TempDir::new("legacy-index-remove");
        let file = directory.path().join(MODPACK_INDEX_FILE);
        std::fs::write(&file, "{}").expect("index");
        remove_legacy_index(&file).expect("removed");
        assert!(!file.exists());
        remove_legacy_index(&file).expect("already gone");
    }
}
