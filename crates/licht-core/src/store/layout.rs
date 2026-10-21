use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::{db, settings};
use crate::{CoreError, Result, SharedCache};

mod database;
mod files;
mod installation;
mod migration;
pub(crate) mod progress;
pub(crate) mod requests;
use progress::{StartupPhase, StartupReporter};

pub async fn initialize_launcher() -> Result<SharedCache> {
    initialize_launcher_with_progress(StartupReporter::default()).await
}

pub async fn initialize_launcher_with_progress(reporter: StartupReporter) -> Result<SharedCache> {
    let home = SharedCache::system()?;
    let legacy = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
        .ok_or(CoreError::DataDirectory)?;
    initialize_at_with_progress(&SharedCache::at(legacy.data_dir()), &home, &reporter).await?;
    Ok(home)
}

#[cfg(test)]
async fn initialize_at(legacy: &SharedCache, home: &SharedCache) -> Result<()> {
    initialize_at_with_progress(legacy, home, &StartupReporter::default()).await
}

async fn initialize_at_with_progress(
    legacy: &SharedCache,
    home: &SharedCache,
    reporter: &StartupReporter,
) -> Result<()> {
    reporter.phase(StartupPhase::Preparing)?;
    let _lock = initialization_lock_with_progress(home, "layout.lock", reporter).await?;

    if !db::db_path(home).exists()
        && (legacy.root().join("settings.json").exists()
            || legacy.root().join("licht.sqlite").exists()
            || legacy.root().join("versions").exists())
    {
        migration::migrate(legacy, home, reporter).await?;
    }
    let game = settings::game_cache(home)?;
    tokio::fs::create_dir_all(game.root()).await?;
    tokio::fs::create_dir_all(game.cache_dir()?).await?;
    tokio::fs::create_dir_all(game.instances_dir()?).await?;
    if !home.root().join("settings.json").exists() {
        settings::save_settings(home, &settings::load_settings(home)?)?;
    }
    reporter.phase(StartupPhase::RestoringCache)?;
    installation::migrate(home, &game, reporter).await?;
    Ok(())
}

pub fn startup_settings() -> Result<crate::Settings> {
    let home = SharedCache::system()?;
    let legacy = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
        .ok_or(CoreError::DataDirectory)?;
    startup_settings_at(&home, &SharedCache::at(legacy.data_dir()))
}

fn startup_settings_at(home: &SharedCache, legacy: &SharedCache) -> Result<crate::Settings> {
    settings::load_settings(if home.root().join("settings.json").is_file() {
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
        settings::save_settings(&legacy, &settings::load_settings(&legacy)?)?;
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
        initialize_at_with_progress(&legacy, &home, &reporter).await
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
