use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use md5::Digest;

use crate::settings::GlobalLaunch;
use crate::{
    CoreError, LaunchEnvironment, Result, SharedCache, Version, apply_global_launch,
    create_natives_directory, extract_natives, launch_command, parse_asset_index,
    reconstruct_assets,
};

#[derive(Debug)]
pub struct OfflineAccount {
    pub username: String,
    pub uuid: String,
}

/// Offline player id used by the game: version 3 UUID of `OfflinePlayer:<name>`,
/// written without hyphens. A namespace UUID would not match.
pub fn offline_account(username: &str) -> Result<OfflineAccount> {
    if username.is_empty() {
        return Err(CoreError::OfflineName);
    }
    let digest = md5::Md5::digest(format!("OfflinePlayer:{username}"));
    let bytes: [u8; 16] = digest.into();
    let uuid = uuid::Builder::from_md5_bytes(bytes)
        .into_uuid()
        .as_simple()
        .to_string();
    Ok(OfflineAccount {
        username: username.to_string(),
        uuid,
    })
}

#[derive(Debug)]
pub struct LaunchArgs {
    pub version_id: String,
    pub username: String,
    pub java: Option<PathBuf>,
    /// Replaces the default `instances/<version>` root when set.
    pub game_directory: Option<PathBuf>,
    pub cache: Option<PathBuf>,
}

pub fn parse_launch_args(args: &[String]) -> Result<LaunchArgs> {
    if args.first().map(String::as_str) != Some("launch") {
        return Err(CoreError::LaunchArgs);
    }

    let mut version_id = None;
    let mut username = None;
    let mut java = None;
    let mut game_directory = None;
    let mut cache = None;
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            return Err(CoreError::LaunchArgs);
        };
        if value.starts_with("--") {
            return Err(CoreError::LaunchArgs);
        }
        match flag.as_str() {
            "--version" => version_id = Some(value.clone()),
            "--username" => username = Some(value.clone()),
            "--java" => java = Some(PathBuf::from(value)),
            "--game-dir" => game_directory = Some(PathBuf::from(value)),
            "--cache" => cache = Some(PathBuf::from(value)),
            _ => return Err(CoreError::LaunchArgs),
        }
    }

    let Some(version_id) = version_id.filter(|id| !id.is_empty()) else {
        return Err(CoreError::LaunchArgs);
    };
    let Some(username) = username else {
        return Err(CoreError::LaunchArgs);
    };
    if username.is_empty() {
        return Err(CoreError::OfflineName);
    }

    Ok(LaunchArgs {
        version_id,
        username,
        java,
        game_directory,
        cache,
    })
}

pub struct OfflineLaunch {
    pub command: Vec<String>,
    /// Set as `APPDATA` for the process. Present when the version uses LaunchWrapper.
    /// This is the instance root. The game reads `<appdata>/.minecraft`.
    pub appdata: Option<PathBuf>,
    /// Directory passed as `--gameDir` and used as the process working directory.
    pub game_directory: PathBuf,
}

/// True when this version starts through LaunchWrapper, including a library
/// that names the injector while the main class is already the client.
fn uses_launchwrapper(version: &Version) -> bool {
    version.main_class == "net.minecraft.launchwrapper.Launch"
        || version
            .libraries
            .iter()
            .any(|library| library.name.starts_with("net.minecraft:launchwrapper:"))
}

/// Extracts natives, then builds the command for an offline account.
/// Does not start the process and does not print the command.
/// `global` is applied to every launch, ahead of the main class.
#[allow(clippy::too_many_arguments)]
pub fn prepare_offline_launch(
    java: &Path,
    cache: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
    account: &OfflineAccount,
    game_directory: &Path,
    global: &GlobalLaunch,
) -> Result<OfflineLaunch> {
    let natives_directory = create_natives_directory(cache, version_id)?;
    extract_natives(cache, version, environment, &natives_directory)?;

    // LaunchWrapper reads `%APPDATA%/.minecraft` or `user.home/.minecraft` before
    // it applies `--gameDir`. The folder name makes both paths the same place.
    let launchwrapper = uses_launchwrapper(version);
    let played = if launchwrapper {
        let played = game_directory.join(".minecraft");
        std::fs::create_dir_all(&played)?;
        played
    } else {
        game_directory.to_path_buf()
    };

    let mut values = BTreeMap::new();
    values.insert("auth_player_name".to_string(), account.username.clone());
    values.insert("auth_uuid".to_string(), account.uuid.clone());
    values.insert("auth_access_token".to_string(), "0".to_string());
    values.insert("user_type".to_string(), "legacy".to_string());
    values.insert("user_properties".to_string(), "{}".to_string());
    values.insert("auth_xuid".to_string(), String::new());
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
