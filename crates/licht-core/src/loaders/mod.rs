//! Mod loader metadata and profile installation (Fabric, Quilt, Forge, NeoForge).

pub(crate) mod fabric;
pub(crate) mod forge;
mod types;

pub use fabric::{install_fabric_like, list_fabric_like_versions};
pub use forge::{install_forge_like, list_forge_like_versions};
pub use types::{LoaderChannel, LoaderPlatform};

use reqwest::Client;

use crate::instance::{InstanceEntry, create_instance};
use crate::settings::GlobalLaunch;
use crate::{Result, SharedCache};
use std::path::Path;

pub(crate) async fn prepare_platform_profile(
    client: &Client,
    cache: &SharedCache,
    platform: LoaderPlatform,
    game_version: &str,
    channel: LoaderChannel,
    loader_version: Option<&str>,
    context: &crate::InstallationContext,
) -> Result<String> {
    use crate::{DownloadComponent, DownloadPhase};
    cache.version_json(game_version)?;
    if platform != LoaderPlatform::Vanilla {
        crate::InstanceSource::Loader {
            platform,
            game_version: game_version.to_string(),
            channel,
            loader_version: loader_version.map(str::to_string),
        }
        .validate()?;
    }
    let component = match platform {
        LoaderPlatform::Vanilla => return Ok(game_version.to_string()),
        LoaderPlatform::Fabric => DownloadComponent::Fabric,
        LoaderPlatform::Quilt => DownloadComponent::Quilt,
        LoaderPlatform::Forge => DownloadComponent::Forge,
        LoaderPlatform::NeoForge => DownloadComponent::NeoForge,
    };
    let context = context.component(component);
    context.stage(DownloadPhase::Preparing).await?;
    match platform {
        LoaderPlatform::Fabric => {
            context
                .metadata(install_fabric_like(
                    client,
                    cache,
                    fabric::FABRIC_META,
                    "fabric-loader",
                    game_version,
                    channel,
                    loader_version,
                ))
                .await
        }
        LoaderPlatform::Quilt => {
            context
                .metadata(install_fabric_like(
                    client,
                    cache,
                    fabric::QUILT_META,
                    "quilt-loader",
                    game_version,
                    channel,
                    loader_version,
                ))
                .await
        }
        LoaderPlatform::Forge => {
            forge::install_forge_like_with_context(
                client,
                cache,
                forge::ForgeFamily::Forge,
                game_version,
                channel,
                loader_version,
                &context,
            )
            .await
        }
        LoaderPlatform::NeoForge => {
            forge::install_forge_like_with_context(
                client,
                cache,
                forge::ForgeFamily::NeoForge,
                game_version,
                channel,
                loader_version,
                &context,
            )
            .await
        }
        LoaderPlatform::Vanilla => Ok(game_version.to_string()),
    }
}

/// Lists loader builds for a game version (newest first).
pub async fn list_loader_versions(
    client: &Client,
    platform: LoaderPlatform,
    game_version: &str,
    channel: LoaderChannel,
) -> Result<Vec<String>> {
    match platform {
        LoaderPlatform::Vanilla => Ok(Vec::new()),
        LoaderPlatform::Fabric => {
            list_fabric_like_versions(client, fabric::FABRIC_META, game_version, channel).await
        }
        LoaderPlatform::Quilt => {
            list_fabric_like_versions(client, fabric::QUILT_META, game_version, channel).await
        }
        LoaderPlatform::Forge => {
            list_forge_like_versions(client, forge::FORGE_METADATA, game_version, channel).await
        }
        LoaderPlatform::NeoForge => {
            list_forge_like_versions(client, forge::NEOFORGE_METADATA, game_version, channel).await
        }
    }
}

/// Creates an instance and installs the selected loader profile when needed.
#[allow(clippy::too_many_arguments)]
pub async fn create_instance_with_platform(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    name: &str,
    platform: LoaderPlatform,
    game_version: &str,
    channel: LoaderChannel,
    loader_version: Option<&str>,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    match platform {
        LoaderPlatform::Vanilla => create_instance(instances_dir, name, game_version, template),
        LoaderPlatform::Fabric => {
            let version_id = install_fabric_like(
                client,
                cache,
                fabric::FABRIC_META,
                "fabric-loader",
                game_version,
                channel,
                loader_version,
            )
            .await?;
            create_instance(instances_dir, name, &version_id, template)
        }
        LoaderPlatform::Quilt => {
            let version_id = install_fabric_like(
                client,
                cache,
                fabric::QUILT_META,
                "quilt-loader",
                game_version,
                channel,
                loader_version,
            )
            .await?;
            create_instance(instances_dir, name, &version_id, template)
        }
        LoaderPlatform::Forge => {
            let version_id = install_forge_like(
                client,
                cache,
                forge::ForgeFamily::Forge,
                game_version,
                channel,
                loader_version,
            )
            .await?;
            create_instance(instances_dir, name, &version_id, template)
        }
        LoaderPlatform::NeoForge => {
            let version_id = install_forge_like(
                client,
                cache,
                forge::ForgeFamily::NeoForge,
                game_version,
                channel,
                loader_version,
            )
            .await?;
            create_instance(instances_dir, name, &version_id, template)
        }
    }
}
