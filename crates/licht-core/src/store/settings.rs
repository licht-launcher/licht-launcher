use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::db::{self, Database};
use super::repositories::settings::SettingsRepository;
use crate::{CoreError, DEFAULT_CONCURRENCY, Result, SharedCache};

const SCHEMA: u32 = 1;
const MIN_CONCURRENCY: u32 = 1;
const MAX_CONCURRENCY: u32 = 16;
const MAX_DOWNLOAD_BYTES_PER_SECOND: u64 = 9_007_199_254_740_991;
pub const MIN_MEMORY_MB: u32 = 512;
pub const MEMORY_STEP_MB: u32 = 64;
const RECOMMENDED_MEMORY_STEP_MB: u32 = 256;
const RECOMMENDED_MIN_MB: u32 = 1024;
const RECOMMENDED_MAX_MB: u32 = 4096;
pub const MIN_XMS_MB: u32 = 512;
const DEFAULT_WIDTH: u32 = 854;
const DEFAULT_HEIGHT: u32 = 480;
const GAME_FOLDER: &str = "Licht Launcher";
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LauncherTheme {
    Light,
    #[default]
    Dark,
    System,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum LauncherLanguage {
    #[default]
    #[serde(rename = "en")]
    English,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema: u32,
    pub theme: LauncherTheme,
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
    #[serde(default)]
    #[ts(type = "number | null")]
    pub download_speed_limit_bytes_per_second: Option<u64>,
    pub data_directory: Option<String>,
    pub disable_custom_title_bar: bool,
    pub automatic_updates: bool,
    pub language: LauncherLanguage,
    pub hardware_acceleration: bool,
    pub minimize_to_tray: bool,
    pub minimize_on_game_start: bool,
    pub restore_on_game_exit: bool,
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
    theme: LauncherTheme,
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
    #[serde(default, rename = "downloadSpeedLimitMiB")]
    download_speed_limit_mib: Option<u32>,
    #[serde(default)]
    download_speed_limit_bytes_per_second: Option<u64>,
    #[serde(default)]
    data_directory: Option<String>,
    #[serde(default)]
    disable_custom_title_bar: bool,
    #[serde(default = "default_true")]
    check_for_updates: bool,
    #[serde(default = "default_true")]
    install_updates_on_restart: bool,
    #[serde(default)]
    automatic_updates: Option<bool>,
    #[serde(default)]
    language: LauncherLanguage,
    #[serde(default = "default_true")]
    hardware_acceleration: bool,
    #[serde(default = "default_true")]
    minimize_to_tray: bool,
    #[serde(default)]
    minimize_on_game_start: bool,
    #[serde(default)]
    restore_on_game_exit: bool,
}

fn default_true() -> bool {
    true
}

fn default_theme() -> LauncherTheme {
    LauncherTheme::Dark
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
    let half = (total_mb / 2) / RECOMMENDED_MEMORY_STEP_MB * RECOMMENDED_MEMORY_STEP_MB;
    half.clamp(RECOMMENDED_MIN_MB, RECOMMENDED_MAX_MB)
        .clamp(MIN_MEMORY_MB, maximum)
}

/// The exact machine RAM endpoint does not need to align with the drag step.
pub fn memory_slider_max(total_mb: u32) -> u32 {
    total_mb.max(MIN_MEMORY_MB)
}

pub(crate) fn validate_memory_allocation(maximum_mb: u32, total_mb: u32) -> Result<()> {
    if !(MIN_MEMORY_MB..=memory_slider_max(total_mb)).contains(&maximum_mb) {
        return Err(CoreError::SettingsMemory);
    }
    Ok(())
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
    (path.is_file() && crate::runtime::is_java_executable(&path)).then_some(path)
}

/// Applies to new user input only. Values already stored are ignored at launch instead,
/// because rejecting them on load or on an internal save would stop the launcher.
fn check_java_executables(settings: &Settings) -> Result<()> {
    let paths = [
        &settings.java25,
        &settings.java21,
        &settings.java17,
        &settings.java8,
    ];
    if paths
        .into_iter()
        .flatten()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .any(|path| !crate::runtime::is_java_executable(Path::new(path)))
    {
        return Err(CoreError::SettingsJava);
    }
    Ok(())
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
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(CoreError::SettingsDirectory);
    }
    if path.components().eq(cache.root().components()) {
        return Ok(None);
    }
    Ok(Some(path.display().to_string()))
}

/// `total_mb` is the machine RAM, probed once by the caller for the whole load or save.
pub(super) fn validated(
    cache: &SharedCache,
    settings: Settings,
    total_mb: u32,
) -> Result<Settings> {
    if settings.schema != SCHEMA {
        return Err(CoreError::SettingsSchema {
            schema: settings.schema,
        });
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
    if settings
        .download_speed_limit_bytes_per_second
        .is_some_and(|value| value == 0 || value > MAX_DOWNLOAD_BYTES_PER_SECOND)
    {
        return Err(CoreError::SettingsDownloadSpeed);
    }
    validate_memory_allocation(settings.max_memory_mb, total_mb)?;
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
        download_speed_limit_bytes_per_second: settings.download_speed_limit_bytes_per_second,
        data_directory: normalize_directory(cache, settings.data_directory.as_deref())?,
        disable_custom_title_bar: settings.disable_custom_title_bar,
        automatic_updates: settings.automatic_updates,
        language: settings.language,
        hardware_acceleration: settings.hardware_acceleration,
        minimize_to_tray: settings.minimize_to_tray,
        minimize_on_game_start: settings.minimize_on_game_start,
        restore_on_game_exit: settings.restore_on_game_exit,
    })
}

fn from_stored(cache: &SharedCache, stored: StoredSettings, total: u32) -> Result<Settings> {
    if stored.schema != SCHEMA {
        return Err(CoreError::SettingsSchema {
            schema: stored.schema,
        });
    }
    let max_memory_mb = match stored.max_memory_mb {
        Some(value) => value,
        None => recommended_max_memory_mb(total),
    };
    // A saved allocation can come from a machine with more RAM.
    let max_memory_mb = max_memory_mb.min(memory_slider_max(total));
    // Legacy files use whole MiB/s. New saves write only the exact byte rate.
    let download_speed_limit_bytes_per_second =
        stored.download_speed_limit_bytes_per_second.or_else(|| {
            stored
                .download_speed_limit_mib
                .map(|value| u64::from(value) * 1024 * 1024)
        });
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
            download_speed_limit_bytes_per_second,
            data_directory: stored.data_directory,
            disable_custom_title_bar: stored.disable_custom_title_bar,
            automatic_updates: stored
                .automatic_updates
                .unwrap_or(stored.check_for_updates && stored.install_updates_on_restart),
            language: stored.language,
            hardware_acceleration: stored.hardware_acceleration,
            minimize_to_tray: stored.minimize_to_tray,
            minimize_on_game_start: stored.minimize_on_game_start,
            restore_on_game_exit: stored.restore_on_game_exit,
        },
        total,
    )
}

fn read_stored(path: &Path) -> Result<StoredSettings> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(CoreError::Settings)
}

/// Preferences kept in `settings.json`, readable before the database opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupPreferences {
    pub theme: LauncherTheme,
    pub language: LauncherLanguage,
    pub hardware_acceleration: bool,
    pub minimize_to_tray: bool,
    pub minimize_on_game_start: bool,
    pub restore_on_game_exit: bool,
    pub automatic_updates: bool,
}

impl From<&Settings> for StartupPreferences {
    fn from(settings: &Settings) -> Self {
        Self {
            theme: settings.theme,
            language: settings.language,
            hardware_acceleration: settings.hardware_acceleration,
            minimize_to_tray: settings.minimize_to_tray,
            minimize_on_game_start: settings.minimize_on_game_start,
            restore_on_game_exit: settings.restore_on_game_exit,
            automatic_updates: settings.automatic_updates,
        }
    }
}

/// What `settings.json` holds. Every other setting lives in the database.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileSettings {
    schema: u32,
    #[serde(flatten)]
    preferences: StartupPreferences,
}

// Every save goes through one temporary file and one database row, so writers in this
// process take turns, and readers wait for a save to finish both parts instead of mixing
// an old file with a new row. The write runs in its own task, so a cancelled caller
// cannot release the lock while the write is still in progress.
static SETTINGS_LOCK: std::sync::LazyLock<std::sync::Arc<tokio::sync::RwLock<()>>> =
    std::sync::LazyLock::new(Default::default);

async fn exclusive_write<T: Send + 'static>(
    work: impl std::future::Future<Output = Result<T>> + Send + 'static,
) -> Result<T> {
    let guard = std::sync::Arc::clone(&SETTINGS_LOCK).write_owned().await;
    tokio::spawn(async move {
        let _guard = guard;
        work.await
    })
    .await
    .map_err(|_| std::io::Error::other("could not join settings write"))?
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| std::io::Error::other("could not join settings file access"))?
}

/// Settings of the profile that owns `database`. A missing `settings.json` means the defaults.
pub async fn load_settings(database: &Database) -> Result<Settings> {
    let _reading = SETTINGS_LOCK.read().await;
    read_settings(database).await
}

/// Callers must hold [`SETTINGS_LOCK`], for reading or writing.
async fn read_settings(database: &Database) -> Result<Settings> {
    Ok(read_with_memory(database).await?.0)
}

/// Reads the settings together with the machine RAM they were checked against.
async fn read_with_memory(database: &Database) -> Result<(Settings, u32)> {
    let file = database.cache().clone();
    let (mut settings, total) = blocking(move || {
        let total = total_memory_mb();
        Ok((load_file_settings(&file, total)?, total))
    })
    .await?;
    let mut connection = database.acquire().await?;
    // Until the first save stores a row, older settings files still carry every value.
    if !SettingsRepository::load_into(&mut connection, &mut settings).await? {
        return Ok((settings, total));
    }
    drop(connection);
    // A saved allocation can come from a machine with more RAM.
    settings.max_memory_mb = settings.max_memory_mb.min(memory_slider_max(total));
    Ok((validated(database.cache(), settings, total)?, total))
}

/// Settings of a folder that has no open pool, such as a legacy profile. A missing
/// database means the file alone, so reading never creates one.
pub(crate) async fn load_settings_at(cache: &SharedCache) -> Result<Settings> {
    if !db::db_path(cache).is_file() {
        let file = cache.clone();
        return blocking(move || load_file_settings(&file, total_memory_mb())).await;
    }
    let database = Database::open(cache).await?;
    let settings = load_settings(&database).await;
    database.close().await;
    settings
}

pub(super) async fn stored_in_database(database: &Database) -> Result<bool> {
    let mut connection = database.acquire().await?;
    SettingsRepository::exists(&mut connection).await
}

pub(super) fn load_startup_preferences(cache: &SharedCache) -> Result<StartupPreferences> {
    Ok(StartupPreferences::from(&load_file_settings(
        cache,
        total_memory_mb(),
    )?))
}

fn load_file_settings(cache: &SharedCache, total: u32) -> Result<Settings> {
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
                download_speed_limit_mib: None,
                download_speed_limit_bytes_per_second: None,
                data_directory: None,
                disable_custom_title_bar: false,
                check_for_updates: true,
                install_updates_on_restart: true,
                automatic_updates: Some(true),
                language: LauncherLanguage::English,
                hardware_acceleration: true,
                minimize_to_tray: true,
                minimize_on_game_start: false,
                restore_on_game_exit: false,
            },
            total,
        );
    }
    from_stored(cache, read_stored(&path)?, total)
}

pub async fn settings_snapshot(database: &Database) -> Result<SettingsSnapshot> {
    let cache = database.cache();
    let (settings, total) = {
        let _reading = SETTINGS_LOCK.read().await;
        read_with_memory(database).await?
    };
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

/// Saves settings from the user, so Java paths must name a Java executable.
pub async fn save_settings(database: &Database, settings: &Settings) -> Result<Settings> {
    check_java_executables(settings)?;
    store_settings(database, settings).await
}

/// Saves settings the launcher already holds (startup, migration, content move).
pub(crate) async fn store_settings(database: &Database, settings: &Settings) -> Result<Settings> {
    exclusive_write(write_settings(database.clone(), settings.clone())).await
}

async fn write_settings(database: Database, settings: Settings) -> Result<Settings> {
    let checked = database.cache().clone();
    let settings = blocking(move || {
        let path = settings_path(&checked);
        let total = total_memory_mb();
        // An unreadable or newer file is left in place rather than overwritten.
        if path.is_file() {
            let _existing = from_stored(&checked, read_stored(&path)?, total)?;
        }
        validated(&checked, settings, total)
    })
    .await?;
    // The row is written first, so an interrupted save keeps the previous startup
    // preferences in the file; the next save writes both again.
    let mut connection = database.acquire().await?;
    SettingsRepository::save(&mut connection, &settings).await?;
    drop(connection);
    let cache = database.cache().clone();
    blocking(move || write_settings_file(&cache, settings)).await
}

fn write_settings_file(cache: &SharedCache, settings: Settings) -> Result<Settings> {
    let path = settings_path(cache);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = FileSettings {
        schema: settings.schema,
        preferences: StartupPreferences::from(&settings),
    };
    let json = serde_json::to_string_pretty(&file).map_err(CoreError::Settings)?;
    // Directory moves must be able to retry after an interrupted settings update.
    let temporary = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    use std::io::Write;
    file.write_all(format!("{json}\n").as_bytes())?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temporary, path)?;
    crate::shared_transfer_limits()
        .bandwidth
        .set_limit(settings.download_speed_limit_bytes_per_second)?;
    Ok(settings)
}

pub async fn set_java_path(database: &Database, major: u32, path: &Path) -> Result<Settings> {
    let database = database.clone();
    let text = Some(path.display().to_string());
    exclusive_write(async move {
        // Read inside the lock so a concurrent save cannot be overwritten with stale values.
        let mut settings = read_settings(&database).await?;
        match major {
            25 => settings.java25 = text,
            21 => settings.java21 = text,
            17 => settings.java17 = text,
            8 => settings.java8 = text,
            _ => return Err(CoreError::SettingsJava),
        }
        write_settings(database, settings).await
    })
    .await
}

/// Game files may live in another folder. When that folder has its own `settings.json`,
/// concurrency comes from it, as before profiles had a database.
async fn concurrency_settings(game: &SharedCache, settings: &Database) -> Result<Settings> {
    if game != settings.cache() && settings_path(game).is_file() {
        load_settings_at(game).await
    } else {
        load_settings(settings).await
    }
}

pub async fn download_concurrency(game: &SharedCache, settings: &Database) -> Result<usize> {
    Ok(concurrency_settings(game, settings)
        .await?
        .download_concurrency as usize)
}

/// Bandwidth, concurrency, and retry for one download batch, from a single settings read.
pub(crate) async fn download_options(
    game: &SharedCache,
    settings: &Database,
) -> Result<crate::DownloadOptions> {
    let saved = concurrency_settings(game, settings).await?;
    let limits = crate::shared_transfer_limits();
    limits
        .bandwidth
        .set_limit(saved.download_speed_limit_bytes_per_second)?;
    Ok(crate::DownloadOptions {
        limits,
        download_concurrency: saved.download_concurrency as usize,
        write_concurrency: saved.write_concurrency as usize,
        retry: crate::DEFAULT_RETRY,
    })
}

/// Next process: game files go to `{dataDirectory}/Licht Launcher/data`.
/// The default game directory is `<launcher>/data`.
pub async fn game_cache(database: &Database) -> Result<SharedCache> {
    let defaults = database.cache();
    let settings = load_settings(database).await?;
    match settings.data_directory {
        Some(path) => Ok(SharedCache::at(
            PathBuf::from(path).join(GAME_FOLDER).join("data"),
        )),
        None => Ok(SharedCache::at(defaults.root().join("data"))),
    }
}

/// One-off pools for tests that read or change a profile outside startup.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    async fn scoped<T>(
        cache: &SharedCache,
        work: impl AsyncFnOnce(&Database) -> Result<T>,
    ) -> Result<T> {
        let database = Database::open(cache).await?;
        let result = work(&database).await;
        database.close().await;
        result
    }

    pub(crate) async fn save_settings_at(
        cache: &SharedCache,
        settings: &Settings,
    ) -> Result<Settings> {
        scoped(cache, async |database| {
            save_settings(database, settings).await
        })
        .await
    }

    pub(crate) async fn game_cache_at(cache: &SharedCache) -> Result<SharedCache> {
        scoped(cache, async |database| game_cache(database).await).await
    }
}

#[cfg(test)]
mod folder_tests {
    use super::*;

    #[tokio::test]
    async fn reading_a_folder_without_a_database_creates_nothing() {
        let cache = SharedCache::at(
            std::env::temp_dir().join(format!("licht-settings-read-{}", std::process::id())),
        );
        let _ = std::fs::remove_dir_all(cache.root());

        let settings = load_settings_at(&cache).await.expect("defaults");

        assert_eq!(settings.schema, SCHEMA);
        assert!(!cache.root().exists());
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;

    #[test]
    fn allocations_are_not_limited_to_sixteen_gb_or_drag_increments() {
        for value in [512, 513, 5001, 32701, 65536] {
            assert!(validate_memory_allocation(value, 65536).is_ok());
        }
        assert!(validate_memory_allocation(511, 65536).is_err());
        assert!(validate_memory_allocation(65537, 65536).is_err());
        assert_eq!(memory_slider_max(32701), 32701);
    }
}
