//! Tauri command handlers. Thin wrappers over `licht-core`.

pub mod accounts;
pub mod downloads;
mod install_progress;
pub mod instances;
pub mod launch;
pub mod modpacks;
mod sessions;
pub mod settings;
pub mod shell;
pub mod versions;

pub use sessions::LaunchSessions;

use std::collections::BTreeMap;
use std::path::PathBuf;

use licht_core::{
    Arch, CoreError, LaunchEnvironment, MANIFEST_REQUEST_TIMEOUT, OsName, SharedCache, game_cache,
};

/// Folders chosen when the process started. A saved data directory is used on the
/// next start, so this process keeps the folders it opened with.
#[derive(Clone)]
pub struct LauncherPaths {
    pub settings: SharedCache,
    pub game: SharedCache,
}

impl LauncherPaths {
    pub fn load() -> Result<Self, CoreError> {
        let settings = SharedCache::system()?;
        let game = game_cache(&settings)?;
        Ok(Self { settings, game })
    }
}

/// Shared HTTP client with a per-request timeout (avoids hanging forever on Mojang).
pub(crate) fn http_client() -> Result<reqwest::Client, CoreError> {
    reqwest::Client::builder()
        .timeout(MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)
}

pub(crate) fn failure(error: CoreError) -> String {
    error.to_string()
}

pub(crate) fn host_environment() -> Result<LaunchEnvironment, CoreError> {
    let os = match std::env::consts::OS {
        "windows" => OsName::Windows,
        "linux" => OsName::Linux,
        "macos" => OsName::Osx,
        _ => return Err(CoreError::LaunchHost),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "x86" => Arch::X86,
        _ => return Err(CoreError::LaunchHost),
    };
    Ok(LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    })
}

pub(crate) fn instances_root(paths: &LauncherPaths) -> Result<PathBuf, CoreError> {
    paths.game.instances_dir()
}
