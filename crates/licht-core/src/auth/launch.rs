use std::collections::BTreeMap;
use std::path::Path;

use crate::auth::types::MicrosoftSession;
use crate::offline::OfflineLaunch;
use crate::settings::GlobalLaunch;
use crate::{
    LaunchEnvironment, Result, SharedCache, Version, apply_global_launch, create_natives_directory,
    extract_natives, launch_command, parse_asset_index, reconstruct_assets,
};

fn uses_launchwrapper(version: &Version) -> bool {
    version.main_class == "net.minecraft.launchwrapper.Launch"
        || version
            .libraries
            .iter()
            .any(|library| library.name.starts_with("net.minecraft:launchwrapper:"))
}

/// Extracts natives, then builds the command for a Microsoft account.
#[allow(clippy::too_many_arguments)]
pub fn prepare_microsoft_launch(
    java: &Path,
    cache: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
    session: &MicrosoftSession,
    game_directory: &Path,
    global: &GlobalLaunch,
) -> Result<OfflineLaunch> {
    let natives_directory = create_natives_directory(cache, version_id)?;
    extract_natives(cache, version, environment, &natives_directory)?;

    let launchwrapper = uses_launchwrapper(version);
    let played = if launchwrapper {
        let played = game_directory.join(".minecraft");
        std::fs::create_dir_all(&played)?;
        played
    } else {
        game_directory.to_path_buf()
    };

    let mut values = BTreeMap::new();
    values.insert("auth_player_name".to_string(), session.profile.name.clone());
    values.insert("auth_uuid".to_string(), session.profile.uuid.clone());
    values.insert(
        "auth_access_token".to_string(),
        session.minecraft_token.clone(),
    );
    values.insert("user_type".to_string(), "msa".to_string());
    values.insert("user_properties".to_string(), "{}".to_string());
    values.insert("auth_xuid".to_string(), session.xuid.clone());
    values.insert("clientid".to_string(), String::new());
    values.insert("version_name".to_string(), version_id.to_string());
    values.insert("version_type".to_string(), "release".to_string());
    values.insert("launcher_name".to_string(), "Licht Launcher".to_string());
    values.insert(
        "launcher_version".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    values.insert("game_directory".to_string(), played.display().to_string());
    values.insert(
        "natives_directory".to_string(),
        natives_directory.display().to_string(),
    );
    values.insert(
        "assets_root".to_string(),
        cache.root().join("assets").display().to_string(),
    );
    values.insert(
        "assets_index_name".to_string(),
        version.asset_index.id.clone(),
    );
    let index_json = std::fs::read_to_string(cache.asset_index(&version.asset_index.id)?)?;
    let index = parse_asset_index(&index_json)?;
    if let Some(assets_dir) = reconstruct_assets(cache, &index, &version.asset_index.id, &played)? {
        values.insert("game_assets".to_string(), assets_dir.display().to_string());
    }

    let mut command = launch_command(java, cache, version_id, version, environment, &values)?;
    let appdata = if launchwrapper {
        command.insert(1, format!("-Duser.home={}", game_directory.display()));
        Some(game_directory.to_path_buf())
    } else {
        None
    };
    apply_global_launch(&mut command, &version.main_class, global);
    Ok(OfflineLaunch {
        command,
        appdata,
        game_directory: played,
    })
}
