//! Modpack discovery (Modrinth / CurseForge) and `.mrpack` import.

mod archive;
mod curseforge;
mod file_hashes;
mod modrinth;
pub(crate) mod mrpack;
mod path_safety;
mod secure_download;
mod types;
pub(crate) mod workspace;

pub use curseforge::{CURSEFORGE_API_KEY_ENV, CURSEFORGE_INDEX_FILE, install_curseforge_files};
pub use mrpack::{
    MODPACK_INDEX_FILE, apply_mrpack_loader, install_mrpack_files, mrpack_download_tasks,
};
pub use types::{ModpackListItem, ModpackSource, ModpackVersionItem};

use reqwest::Client;

use crate::instance::{InstanceEntry, InstanceSource, instance_entry};
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result, SharedCache};
use std::path::Path;

/// Searches Modrinth always; CurseForge when `LICHT_CURSEFORGE_API_KEY` is set.
pub async fn search_modpacks(client: &Client, query: &str) -> Result<Vec<ModpackListItem>> {
    let mut items = modrinth::search_modrinth(client, query).await?;
    match curseforge::search_curseforge(client, query).await {
        Ok(mut curse) => items.append(&mut curse),
        Err(CoreError::CurseforgeKeyMissing) => {}
        Err(error) => return Err(error),
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.downloads));
    Ok(items)
}

pub async fn list_modpack_versions(
    client: &Client,
    source: ModpackSource,
    project_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    match source {
        ModpackSource::Modrinth => modrinth::list_modrinth_versions(client, project_id).await,
        ModpackSource::Curseforge => curseforge::list_curseforge_versions(client, project_id).await,
    }
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
        match source {
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
    }
    context.stage(crate::DownloadPhase::Extracting).await?;
    let bytes = std::fs::read(&archive)?;
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
    if zip.file_names().any(|name| name == "modrinth.index.json") {
        Ok(ModpackSource::Modrinth)
    } else if zip.file_names().any(|name| name == "manifest.json") {
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
