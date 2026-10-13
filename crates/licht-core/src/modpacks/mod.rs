//! Modpack discovery (Modrinth / CurseForge) and `.mrpack` import.

mod curseforge;
mod file_hashes;
mod modrinth;
mod mrpack;
mod path_safety;
mod secure_download;
mod types;

pub use curseforge::{CURSEFORGE_API_KEY_ENV, CURSEFORGE_INDEX_FILE, install_curseforge_files};
pub use mrpack::{
    MODPACK_INDEX_FILE, apply_mrpack_loader, import_mrpack_bytes, install_mrpack_files,
    mrpack_download_tasks,
};
pub use types::{ModpackListItem, ModpackSource, ModpackVersionItem};

use reqwest::Client;

use crate::instance::icon::download_instance_icon;
use crate::instance::{InstanceEntry, instance_entry};
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
    let (_file_name, bytes) = match source {
        ModpackSource::Modrinth => {
            modrinth::download_modrinth_mrpack(client, project_id, version_id).await?
        }
        ModpackSource::Curseforge => {
            curseforge::download_curseforge_pack(client, project_id, version_id).await?
        }
    };
    let entry = match source {
        ModpackSource::Modrinth => {
            let entry = import_mrpack_bytes(instances_dir, name, &bytes, template)?;
            apply_mrpack_loader(client, cache, instances_dir, &entry).await?
        }
        ModpackSource::Curseforge => {
            curseforge::import_curseforge_pack(client, instances_dir, name, &bytes, template)
                .await?
        }
    };
    let dir = instances_dir.join(&entry.folder);
    if let Some(url) = icon_url {
        let client = client.clone();
        let dir = dir.clone();
        let url = url.to_string();
        tokio::spawn(async move {
            if let Err(error) = download_instance_icon(&client, &dir, &url).await {
                tracing::warn!(error = %error, "instance icon download failed");
            }
        });
    }
    instance_entry(&dir, &entry.folder)
}
