//! Modpack discovery (Modrinth / CurseForge) and `.mrpack` import.

mod archive;
mod curseforge;
mod description;
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
pub use types::{
    ModpackDetails, ModpackListItem, ModpackSearchPage, ModpackSource, ModpackVersionItem,
};

use types::SourcePage;

use reqwest::Client;

use crate::instance::{InstanceEntry, InstanceSource, instance_entry};
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result, SharedCache};
use std::path::Path;

/// Results each source contributes to one page of the merged search.
pub const SEARCH_PAGE_SIZE_PER_SOURCE: u32 = 10;

/// Searches Modrinth always; CurseForge when `LICHT_CURSEFORGE_API_KEY` is set.
///
/// Both sources are asked for the same 1-based `page`, and that page is sorted
/// by downloads. The order is exact within a page, not across pages.
pub async fn search_modpacks(client: &Client, query: &str, page: u32) -> Result<ModpackSearchPage> {
    if page == 0 {
        return Err(CoreError::ModpackMeta {
            detail: "search pages start at 1".into(),
        });
    }
    let size = SEARCH_PAGE_SIZE_PER_SOURCE;
    let offset = u64::from(page - 1) * u64::from(size);
    let curseforge = async {
        // Past CurseForge's search window the API errors, so that source is just done.
        if offset + u64::from(size) > curseforge::CURSEFORGE_SEARCH_WINDOW {
            return Ok(None);
        }
        match curseforge::search_curseforge(client, query, offset, size).await {
            Ok(page) => Ok(Some(page)),
            Err(CoreError::CurseforgeKeyMissing) => Ok(None),
            Err(error) => Err(error),
        }
    };
    let (modrinth, curseforge) = tokio::join!(
        modrinth::search_modrinth(client, query, offset, size),
        curseforge,
    );
    let mut sources = vec![modrinth?];
    if let Some(mut curseforge) = curseforge? {
        curseforge.total = curseforge.total.min(curseforge::CURSEFORGE_SEARCH_WINDOW);
        sources.push(curseforge);
    }
    Ok(merge_search_pages(page, sources))
}

fn merge_search_pages(page: u32, sources: Vec<SourcePage>) -> ModpackSearchPage {
    let largest = sources.iter().map(|source| source.total).max().unwrap_or(0);
    let page_count = largest.div_ceil(u64::from(SEARCH_PAGE_SIZE_PER_SOURCE));
    let mut items: Vec<ModpackListItem> = sources
        .into_iter()
        .flat_map(|source| source.items)
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.downloads));
    ModpackSearchPage {
        items,
        page,
        page_count: u32::try_from(page_count).unwrap_or(u32::MAX),
    }
}

/// Store page data for one modpack, with its description already sanitized.
pub async fn get_modpack(
    client: &Client,
    source: ModpackSource,
    project_id: &str,
) -> Result<ModpackDetails> {
    match source {
        ModpackSource::Modrinth => modrinth::get_modrinth_project(client, project_id).await,
        ModpackSource::Curseforge => curseforge::get_curseforge_mod(client, project_id).await,
    }
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
mod search_tests {
    use super::*;

    fn item(name: &str, source: ModpackSource, downloads: u64) -> ModpackListItem {
        ModpackListItem {
            id: name.into(),
            source,
            name: name.into(),
            description: String::new(),
            downloads,
            icon_url: None,
            author: String::new(),
        }
    }

    #[test]
    fn merges_both_sources_sorted_by_downloads() {
        let page = merge_search_pages(
            2,
            vec![
                SourcePage {
                    items: vec![item("a", ModpackSource::Modrinth, 5)],
                    total: 25,
                },
                SourcePage {
                    items: vec![item("b", ModpackSource::Curseforge, 9)],
                    total: 12,
                },
            ],
        );
        let names: Vec<&str> = page.items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, vec!["b", "a"]);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_count, 3);
    }

    #[test]
    fn no_hits_means_no_pages() {
        let page = merge_search_pages(
            1,
            vec![SourcePage {
                items: vec![],
                total: 0,
            }],
        );
        assert!(page.items.is_empty());
        assert_eq!(page.page_count, 0);
    }

    #[tokio::test]
    async fn page_zero_is_rejected_before_any_request() {
        let client = Client::new();
        assert!(search_modpacks(&client, "", 0).await.is_err());
    }
}
