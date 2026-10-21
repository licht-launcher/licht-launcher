use std::path::Path;

use crate::instance::{InstanceEntry, update_instance_version};
use crate::loaders::{LoaderChannel, LoaderPlatform};
use crate::{CoreError, Result, SharedCache};
use reqwest::Client;

/// Installs the loader declared in `dependencies` and updates the instance version id.
pub async fn apply_mrpack_loader(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
) -> Result<InstanceEntry> {
    apply_mrpack_loader_with_context(
        database,
        client,
        cache,
        instances_dir,
        entry,
        &crate::InstallationContext::default(),
    )
    .await
}

pub(crate) async fn apply_mrpack_loader_with_context(
    database: &crate::DatabaseClient,
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    context: &crate::InstallationContext,
) -> Result<InstanceEntry> {
    let deps = database
        .modpack_dependencies(instances_dir, &entry.folder)
        .await?;
    if deps.is_empty() {
        return Ok(entry.clone());
    }
    let dependency = |name: &str| {
        deps.iter()
            .find(|(key, _)| key == name)
            .map(|(_, version)| version.as_str())
    };
    let game_version = dependency("minecraft")
        .or(entry.version_id.as_deref())
        .ok_or(CoreError::ModpackArchive)?;

    let selected = [
        ("fabric-loader", LoaderPlatform::Fabric),
        ("quilt-loader", LoaderPlatform::Quilt),
        ("forge", LoaderPlatform::Forge),
        ("neoforge", LoaderPlatform::NeoForge),
    ]
    .into_iter()
    .find_map(|(key, platform)| dependency(key).map(|version| (platform, version)));
    let Some((platform, version)) = selected else {
        return Ok(entry.clone());
    };
    let loader_id = crate::loaders::prepare_platform_profile(
        client,
        cache,
        platform,
        game_version,
        LoaderChannel::Manual,
        Some(version),
        context,
    )
    .await?;
    update_instance_version(instances_dir, &entry.folder, &loader_id)
}
