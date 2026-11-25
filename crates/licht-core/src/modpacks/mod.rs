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

fn linked_modpack(details: ModpackDetails, version: PackVersion) -> LinkedModpack {
    LinkedModpack {
        name: details.name,
        author: Some(details.author).filter(|author| !author.trim().is_empty()),
        version_name: version.name,
        published_at: version.published_at,
    }
}

/// Keeps the store's details of the pack on the instance. They only feed the UI, so a
/// store that cannot describe the pack does not stop the install.
async fn record_store_details(
    database: &crate::DatabaseClient,
    client: &Client,
    instances_dir: &Path,
    folder: &str,
    source: ModpackSource,
    project_id: &str,
    version: PackVersion,
) -> Result<()> {
    let details = match get_modpack(client, source, project_id).await {
        Ok(details) => details,
        Err(error) => {
            tracing::warn!(%error, "could not read the modpack's store details");
            return Ok(());
        }
    };
    crate::instance::record_linked_modpack(
        database,
        instances_dir,
        folder,
        linked_modpack(details, version),
    )
    .await
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
        false,
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
        true,
    )
    .await
}

/// Clears the content folders a reinstall replaces, with anything the user added to
/// them. Worlds and other folders stay. The pack's stored rows are kept: the import
/// replaces them, and until it does, Repair can still restore the previous files.
fn reset_pack_content(dir: &Path) -> Result<()> {
    for name in ["mods", "resourcepacks", "shaderpacks"] {
        // `remove_dir_all` removes a linked folder's link, not the folder it points to.
        match std::fs::remove_dir_all(dir.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

// One flag separates a first install from a reinstall; the steps are otherwise shared.
#[allow(clippy::too_many_arguments)]
async fn install_modpack(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    selection: &InstanceSource,
    context: &crate::InstallationContext,
    reset_content: bool,
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
        record_store_details(
            database,
            client,
            instances_dir,
            &entry.folder,
            *source,
            project_id,
            version,
        )
        .await?;
    }
    context.stage(crate::DownloadPhase::Extracting).await?;
    if reset_content {
        reset_pack_content(&dir)?;
    }
    let bytes = std::fs::read(&archive)?;
    if remote.is_none()
        && let Some(details) = archive_details::archive_details(*source, &bytes)
    {
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

    #[test]
    fn keeps_the_pack_name_author_and_installed_version() {
        let version = PackVersion {
            name: "15.0.0-beta.3".into(),
            published_at: Some("2026-11-01T10:00:00Z".into()),
        };
        assert_eq!(
            linked_modpack(details("robotkoer"), version.clone()),
            LinkedModpack {
                name: "Fabulously Optimized".into(),
                author: Some("robotkoer".into()),
                version_name: "15.0.0-beta.3".into(),
                published_at: Some("2026-11-01T10:00:00Z".into()),
            }
        );
        assert_eq!(linked_modpack(details("  "), version).author, None);
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
