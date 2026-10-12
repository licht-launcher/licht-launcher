use std::fs;
use std::path::Path;

use reqwest::Client;
use serde_json::Value;

use super::MODPACK_INDEX_FILE;
use crate::instance::{InstanceEntry, update_instance_version};
use crate::loaders::LoaderChannel;
use crate::loaders::fabric::{self, install_fabric_like};
use crate::loaders::forge::{self, install_forge_like};
use crate::{CoreError, Result, SharedCache};

/// Installs the loader declared in `dependencies` and updates the instance version id.
pub async fn apply_mrpack_loader(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    entry: &InstanceEntry,
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
        .unwrap_or(entry.version_id.as_str());

    let loader_id = if let Some(version) = deps.get("fabric-loader").and_then(Value::as_str) {
        Some(
            install_fabric_like(
                client,
                cache,
                fabric::FABRIC_META,
                "fabric-loader",
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("quilt-loader").and_then(Value::as_str) {
        Some(
            install_fabric_like(
                client,
                cache,
                fabric::QUILT_META,
                "quilt-loader",
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("forge").and_then(Value::as_str) {
        Some(
            install_forge_like(
                client,
                cache,
                forge::ForgeFamily::Forge,
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else if let Some(version) = deps.get("neoforge").and_then(Value::as_str) {
        Some(
            install_forge_like(
                client,
                cache,
                forge::ForgeFamily::NeoForge,
                game_version,
                LoaderChannel::Manual,
                Some(version),
            )
            .await?,
        )
    } else {
        None
    };

    let Some(loader_id) = loader_id else {
        return Ok(entry.clone());
    };
    update_instance_version(instances_dir, &entry.folder, &loader_id)
}
