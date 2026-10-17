use std::fs;
use std::path::Path;

use reqwest::Client;
use serde_json::Value;

use super::MODPACK_INDEX_FILE;
use crate::instance::{InstanceEntry, update_instance_version};
use crate::loaders::{LoaderChannel, LoaderPlatform};
use crate::{CoreError, Result, SharedCache};

/// Installs the loader declared in `dependencies` and updates the instance version id.
pub async fn apply_mrpack_loader(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
) -> Result<InstanceEntry> {
    apply_mrpack_loader_with_context(
        client,
        cache,
        instances_dir,
        entry,
        &crate::InstallationContext::default(),
    )
    .await
}

pub(crate) async fn apply_mrpack_loader_with_context(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
    context: &crate::InstallationContext,
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
        .or(entry.version_id.as_deref())
        .ok_or(CoreError::ModpackArchive)?;

    let selected = [
        ("fabric-loader", LoaderPlatform::Fabric),
        ("quilt-loader", LoaderPlatform::Quilt),
        ("forge", LoaderPlatform::Forge),
        ("neoforge", LoaderPlatform::NeoForge),
    ]
    .into_iter()
    .find_map(|(key, platform)| {
        deps.get(key)
            .and_then(Value::as_str)
            .map(|version| (platform, version))
    });
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
