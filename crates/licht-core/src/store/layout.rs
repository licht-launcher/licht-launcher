use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::db::{self, Database};
use super::settings;
use crate::{CoreError, Result, SharedCache};

mod cleanup;
mod database;
mod files;
mod installation;
mod migration;
pub(crate) mod progress;
mod relocation;
pub(crate) mod requests;
use progress::{StartupPhase, StartupReporter};

/// Prepares the profile and returns the database pool every later caller shares.
pub async fn initialize_launcher() -> Result<Database> {
    initialize_launcher_with_progress(StartupReporter::default()).await
}

pub async fn initialize_launcher_with_progress(reporter: StartupReporter) -> Result<Database> {
    let home = SharedCache::system()?;
    if std::env::var_os("LICHT_DATA_DIR").is_some() {
        return initialize_at_with_progress(None, &home, &reporter).await;
    }
    let legacy = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
        .ok_or(CoreError::DataDirectory)?;
    initialize_at_with_progress(Some(&SharedCache::at(legacy.data_dir())), &home, &reporter).await
}

#[cfg(test)]
async fn initialize_at(legacy: &SharedCache, home: &SharedCache) -> Result<()> {
    initialize_at_with_progress(Some(legacy), home, &StartupReporter::default())
        .await?
        .close()
        .await;
    Ok(())
}

async fn initialize_at_with_progress(
    legacy: Option<&SharedCache>,
    home: &SharedCache,
    reporter: &StartupReporter,
) -> Result<Database> {
    reporter.phase(StartupPhase::Preparing)?;
    let _lock = initialization_lock_with_progress(home, "layout.lock", reporter).await?;

    if let Some(legacy) = legacy
        && !db::db_path(home).exists()
        && (legacy.root().join("settings.json").exists()
            || legacy.root().join("licht.sqlite").exists()
            || legacy.root().join("versions").exists())
    {
        // The migration publishes licht.db by renaming a staged file, so the profile
        // pool opens only after it returns.
        migration::migrate(legacy, home, reporter).await?;
    }
    let database = Database::open(home).await?;
    if let Err(error) = prepare_profile(&database, legacy, reporter).await {
        database.close().await;
        return Err(error);
    }
    Ok(database)
}

async fn prepare_profile(
    database: &Database,
    legacy: Option<&SharedCache>,
    reporter: &StartupReporter,
) -> Result<()> {
    // Older profiles keep every setting in settings.json until one save moves them.
    // The content move checkpoint lives in the same row, so it must exist first.
    if !database.cache().root().join("settings.json").exists()
        || !settings::stored_in_database(database).await?
    {
        settings::store_settings(database, &settings::load_settings(database).await?).await?;
    }
    relocation::apply(database, reporter).await?;
    if let Some(legacy) = legacy
        && cleanup::pending(legacy, database.cache())
        && let Err(error) = cleanup::remove_legacy(legacy, database, reporter).await
    {
        // The launcher already uses the migrated copy, so a later startup retries this.
        tracing::warn!(error = %error, "Could not remove the migrated legacy profile");
    }
    let game = settings::game_cache(database).await?;
    tokio::fs::create_dir_all(game.root()).await?;
    tokio::fs::create_dir_all(game.cache_dir()?).await?;
    tokio::fs::create_dir_all(game.instances_dir()?).await?;
    reporter.phase(StartupPhase::RestoringCache)?;
    installation::migrate(database, &game, reporter).await
}

pub fn startup_preferences() -> Result<settings::StartupPreferences> {
    let home = SharedCache::system()?;
    if std::env::var_os("LICHT_DATA_DIR").is_some() {
        return settings::load_startup_preferences(&home);
    }
    let legacy = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
        .ok_or(CoreError::DataDirectory)?;
    startup_preferences_at(&home, &SharedCache::at(legacy.data_dir()))
}

fn startup_preferences_at(
    home: &SharedCache,
    legacy: &SharedCache,
) -> Result<settings::StartupPreferences> {
    settings::load_startup_preferences(if home.root().join("settings.json").is_file() {
        home
    } else {
        legacy
    })
}

pub async fn run_startup_migration_preview(fail: bool, reporter: StartupReporter) -> Result<()> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|error| std::io::Error::other(error.to_string()))?;
    let root = std::env::temp_dir()
        .join("licht-launcher-startup-preview")
        .join(crate::hashes::hex_encode(&nonce));
    let legacy = SharedCache::at(root.join("legacy").join("data"));
    let home = SharedCache::at(root.join("destination"));
    let result = async {
        std::fs::create_dir_all(legacy.root().join("versions"))?;
        let database = Database::open(&legacy).await?;
        let saved = async {
            settings::store_settings(&database, &settings::load_settings(&database).await?).await
        }
        .await;
        database.close().await;
        saved?;
        let instance = legacy.instance_dir("Preview")?;
        std::fs::create_dir_all(instance.join("mods"))?;
        for index in 0..100 {
            std::fs::write(
                instance.join("mods").join(format!("fixture-{index}.jar")),
                vec![b'x'; 64 * 1024],
            )?;
        }
        if fail {
            std::fs::create_dir_all(home.root().join("instances").join("Preview").join("mods"))?;
            std::fs::write(
                home.root()
                    .join("instances")
                    .join("Preview")
                    .join("mods")
                    .join("fixture-0.jar"),
                b"conflict",
            )?;
        }
        initialize_at_with_progress(Some(&legacy), &home, &reporter)
            .await?
            .close()
            .await;
        Ok(())
    }
    .await;
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    result
}

pub(crate) async fn initialization_lock(home: &SharedCache, name: &str) -> Result<File> {
    initialization_lock_with_progress(home, name, &StartupReporter::default()).await
}

async fn initialization_lock_with_progress(
    home: &SharedCache,
    name: &str,
    reporter: &StartupReporter,
) -> Result<File> {
    let cache = home.root().join("cache");
    tokio::fs::create_dir_all(&cache).await?;
    let lock_path = cache.join(name);
    let reporter = reporter.clone();
    tokio::task::spawn_blocking(move || -> Result<File> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        loop {
            reporter.checkpoint()?;
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
                Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
            }
        }
        Ok(lock)
    })
    .await
    .map_err(|_| std::io::Error::other("could not join database initialization lock"))?
}

#[derive(Clone)]
struct Relocation {
    source: PathBuf,
    destination: PathBuf,
}

fn relocate(path: &Path, mappings: &[Relocation]) -> PathBuf {
    if mappings
        .iter()
        .any(|mapping| path.starts_with(&mapping.destination))
    {
        return path.to_path_buf();
    }
    for mapping in mappings {
        if let Ok(relative) = path.strip_prefix(&mapping.source) {
            return mapping.destination.join(relative);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests;
