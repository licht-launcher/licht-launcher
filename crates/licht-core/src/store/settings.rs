use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CoreError, DEFAULT_CONCURRENCY, Result, SharedCache};

const SCHEMA: u32 = 1;
const MIN_CONCURRENCY: u32 = 1;
const MAX_CONCURRENCY: u32 = 16;
pub const MIN_MEMORY_MB: u32 = 512;
pub const MEMORY_STEP_MB: u32 = 256;
const MAX_MEMORY_CAP_MB: u32 = 16384;
const RECOMMENDED_MIN_MB: u32 = 1024;
const RECOMMENDED_MAX_MB: u32 = 4096;
pub const MIN_XMS_MB: u32 = 512;
const DEFAULT_WIDTH: u32 = 854;
const DEFAULT_HEIGHT: u32 = 480;
const GAME_FOLDER: &str = "Licht Launcher";
const THEME: &str = "dark";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema: u32,
    pub theme: String,
    pub username: String,
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub max_memory_mb: u32,
    pub jvm_arguments: Vec<String>,
    pub java25: Option<String>,
    pub java21: Option<String>,
    pub java17: Option<String>,
    pub java8: Option<String>,
    pub download_concurrency: u32,
    pub write_concurrency: u32,
    pub data_directory: Option<String>,
    pub disable_custom_title_bar: bool,
    pub check_for_updates: bool,
    pub install_updates_on_restart: bool,
}

/// What the settings screen shows. `application_directory` is filled even when
/// `settings.data_directory` is empty, so the field is never blank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub settings: Settings,
    pub application_directory: String,
    pub memory_total_mb: u32,
    pub memory_recommended_mb: u32,
    pub memory_minimum_mb: u32,
    pub memory_maximum_mb: u32,
    pub memory_step_mb: u32,
}

/// Memory, arguments, and window applied to one launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalLaunch {
    pub max_memory_mb: u32,
    pub jvm_arguments: Vec<String>,
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    schema: u32,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_username")]
    username: String,
    #[serde(default)]
    fullscreen: bool,
    #[serde(default = "default_width")]
    width: Option<u32>,
    #[serde(default = "default_height")]
    height: Option<u32>,
    #[serde(default)]
    max_memory_mb: Option<u32>,
    #[serde(default)]
    jvm_arguments: Vec<String>,
    #[serde(default)]
    java25: Option<String>,
    #[serde(default)]
    java21: Option<String>,
    #[serde(default)]
    java17: Option<String>,
    #[serde(default)]
    java8: Option<String>,
    #[serde(default = "default_concurrency_value")]
    download_concurrency: u32,
    #[serde(default = "default_concurrency_value")]
    write_concurrency: u32,
    #[serde(default)]
    data_directory: Option<String>,
    #[serde(default)]
    disable_custom_title_bar: bool,
    #[serde(default = "default_true")]
    check_for_updates: bool,
    #[serde(default = "default_true")]
    install_updates_on_restart: bool,
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    THEME.to_string()
}

fn default_username() -> String {
    "Steve".to_string()
}

fn default_width() -> Option<u32> {
    Some(DEFAULT_WIDTH)
}

fn default_height() -> Option<u32> {
    Some(DEFAULT_HEIGHT)
}

fn default_concurrency_value() -> u32 {
    u32::try_from(DEFAULT_CONCURRENCY).unwrap_or(8)
}

fn settings_path(cache: &SharedCache) -> PathBuf {
    cache.root().join("settings.json")
}

/// Half of the machine RAM, rounded down to 256 MB, then kept between 1024 and
/// 4096. Never above the slider maximum for that machine.
pub fn recommended_max_memory_mb(total_mb: u32) -> u32 {
    let maximum = memory_slider_max(total_mb);
    let half = (total_mb / 2) / MEMORY_STEP_MB * MEMORY_STEP_MB;
    half.clamp(RECOMMENDED_MIN_MB, RECOMMENDED_MAX_MB)
        .clamp(MIN_MEMORY_MB, maximum)
}

/// Slider upper bound: the machine RAM, at most 16384, on a 256 MB step.
pub fn memory_slider_max(total_mb: u32) -> u32 {
    let capped = total_mb.clamp(MIN_MEMORY_MB, MAX_MEMORY_CAP_MB);
    let stepped = capped / MEMORY_STEP_MB * MEMORY_STEP_MB;
    stepped.max(MIN_MEMORY_MB)
}

pub fn total_memory_mb() -> u32 {
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::everything()),
    );
    let bytes = system.total_memory();
    let mb = u32::try_from(bytes / 1024 / 1024).unwrap_or(2048);
    if mb == 0 { 2048 } else { mb }
}

pub fn global_launch(settings: &Settings) -> GlobalLaunch {
    GlobalLaunch {
        max_memory_mb: settings.max_memory_mb,
        jvm_arguments: settings.jvm_arguments.clone(),
        fullscreen: settings.fullscreen,
        width: settings.width,
        height: settings.height,
    }
}

pub fn configured_java(settings: &Settings, major: u32) -> Option<PathBuf> {
    let text = match major {
        25 => settings.java25.as_deref(),
        21 => settings.java21.as_deref(),
        17 => settings.java17.as_deref(),
        8 => settings.java8.as_deref(),
        _ => None,
    }?;
    let path = PathBuf::from(text);
    path.is_file().then_some(path)
}

fn clean_java(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.contains('\0') {
        return Err(CoreError::SettingsJava);
    }
    Ok(Some(value.to_string()))
}

fn normalize_directory(cache: &SharedCache, value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.contains('\0') {
        return Err(CoreError::SettingsDirectory);
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(CoreError::SettingsDirectory);
    }
    if path.components().eq(cache.root().components()) {
        return Ok(None);
    }
    Ok(Some(path.display().to_string()))
}

pub(super) fn validated(cache: &SharedCache, settings: Settings) -> Result<Settings> {
    if settings.schema != SCHEMA {
        return Err(CoreError::SettingsSchema {
            schema: settings.schema,
        });
    }
    if settings.theme != THEME {
        return Err(CoreError::SettingsTheme);
    }
    let username = settings.username.trim().to_string();
    if username.is_empty() {
        return Err(CoreError::OfflineName);
    }
    if !(MIN_CONCURRENCY..=MAX_CONCURRENCY).contains(&settings.download_concurrency) {
        return Err(CoreError::SettingsConcurrency);
    }
    if !(MIN_CONCURRENCY..=MAX_CONCURRENCY).contains(&settings.write_concurrency) {
        return Err(CoreError::SettingsWriteConcurrency);
    }
    if settings.max_memory_mb < MIN_MEMORY_MB
        || settings.max_memory_mb > MAX_MEMORY_CAP_MB
        || !settings.max_memory_mb.is_multiple_of(MEMORY_STEP_MB)
    {
        return Err(CoreError::SettingsMemory);
    }
    if matches!(settings.width, Some(0)) || matches!(settings.height, Some(0)) {
        return Err(CoreError::SettingsWindow);
    }
    Ok(Settings {
        schema: SCHEMA,
        theme: settings.theme,
        username,
        fullscreen: settings.fullscreen,
        width: settings.width,
        height: settings.height,
        max_memory_mb: settings.max_memory_mb,
        jvm_arguments: settings
            .jvm_arguments
            .into_iter()
            .filter(|argument| !argument.is_empty())
            .collect(),
        java25: clean_java(settings.java25.as_deref())?,
        java21: clean_java(settings.java21.as_deref())?,
        java17: clean_java(settings.java17.as_deref())?,
        java8: clean_java(settings.java8.as_deref())?,
        download_concurrency: settings.download_concurrency,
        write_concurrency: settings.write_concurrency,
        data_directory: normalize_directory(cache, settings.data_directory.as_deref())?,
        disable_custom_title_bar: settings.disable_custom_title_bar,
        check_for_updates: settings.check_for_updates,
        install_updates_on_restart: settings.install_updates_on_restart,
    })
}

fn from_stored(cache: &SharedCache, stored: StoredSettings) -> Result<Settings> {
    if stored.schema != SCHEMA {
        return Err(CoreError::SettingsSchema {
            schema: stored.schema,
        });
    }
    let max_memory_mb = match stored.max_memory_mb {
        Some(value) => value,
        None => recommended_max_memory_mb(total_memory_mb()),
    };
    validated(
        cache,
        Settings {
            schema: stored.schema,
            theme: stored.theme,
            username: stored.username,
            fullscreen: stored.fullscreen,
            width: stored.width,
            height: stored.height,
            max_memory_mb,
            jvm_arguments: stored.jvm_arguments,
            java25: stored.java25,
            java21: stored.java21,
            java17: stored.java17,
            java8: stored.java8,
            download_concurrency: stored.download_concurrency,
            write_concurrency: stored.write_concurrency,
            data_directory: stored.data_directory,
            disable_custom_title_bar: stored.disable_custom_title_bar,
            check_for_updates: stored.check_for_updates,
            install_updates_on_restart: stored.install_updates_on_restart,
        },
    )
}

fn read_stored(path: &Path) -> Result<StoredSettings> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(CoreError::Settings)
}

/// Missing `settings.json` means the defaults. The file is created only when saved.
pub fn load_settings(cache: &SharedCache) -> Result<Settings> {
    let path = settings_path(cache);
    if !path.is_file() {
        return from_stored(
            cache,
            StoredSettings {
                schema: SCHEMA,
                theme: default_theme(),
                username: default_username(),
                fullscreen: false,
                width: default_width(),
                height: default_height(),
                max_memory_mb: None,
                jvm_arguments: Vec::new(),
                java25: None,
                java21: None,
                java17: None,
                java8: None,
                download_concurrency: default_concurrency_value(),
                write_concurrency: default_concurrency_value(),
                data_directory: None,
                disable_custom_title_bar: false,
                check_for_updates: true,
                install_updates_on_restart: true,
            },
        );
    }
    from_stored(cache, read_stored(&path)?)
}

pub fn settings_snapshot(cache: &SharedCache) -> Result<SettingsSnapshot> {
    let settings = load_settings(cache)?;
    let total = total_memory_mb();
    let application_directory = match &settings.data_directory {
        Some(path) => path.clone(),
        None => cache.root().display().to_string(),
    };
    Ok(SettingsSnapshot {
        settings,
        application_directory,
        memory_total_mb: total,
        memory_recommended_mb: recommended_max_memory_mb(total),
        memory_minimum_mb: MIN_MEMORY_MB,
        memory_maximum_mb: memory_slider_max(total),
        memory_step_mb: MEMORY_STEP_MB,
    })
}

pub fn save_settings(cache: &SharedCache, settings: &Settings) -> Result<Settings> {
    let path = settings_path(cache);
    if path.is_file() {
        let _existing = from_stored(cache, read_stored(&path)?)?;
    }
    let settings = validated(cache, settings.clone())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(CoreError::Settings)?;
    std::fs::write(path, format!("{json}\n"))?;
    Ok(settings)
}

pub fn set_java_path(cache: &SharedCache, major: u32, path: &Path) -> Result<Settings> {
    let mut settings = load_settings(cache)?;
    let text = Some(path.display().to_string());
    match major {
        25 => settings.java25 = text,
        21 => settings.java21 = text,
        17 => settings.java17 = text,
        8 => settings.java8 = text,
        _ => return Err(CoreError::SettingsJava),
    }
    save_settings(cache, &settings)
}

fn concurrency_source<'a>(game: &'a SharedCache, settings: &'a SharedCache) -> &'a SharedCache {
    if settings_path(game).is_file() {
        game
    } else {
        settings
    }
}

/// Game files may live in another folder. When that folder has no
/// `settings.json`, concurrency still comes from the settings folder.
pub fn download_concurrency(game: &SharedCache, settings: &SharedCache) -> Result<usize> {
    Ok(load_settings(concurrency_source(game, settings))?.download_concurrency as usize)
}

/// How many files may be written to disk at once across all installs.
pub fn write_concurrency(game: &SharedCache, settings: &SharedCache) -> Result<usize> {
    Ok(load_settings(concurrency_source(game, settings))?.write_concurrency as usize)
}

/// Next process: game files go to `{dataDirectory}/Licht Launcher/data`.
/// The default game directory is `<launcher>/data`.
pub fn game_cache(defaults: &SharedCache) -> Result<SharedCache> {
    let settings = load_settings(defaults)?;
    match settings.data_directory {
        Some(path) => Ok(SharedCache::at(
            PathBuf::from(path).join(GAME_FOLDER).join("data"),
        )),
        None => Ok(SharedCache::at(defaults.root().join("data"))),
    }
}
