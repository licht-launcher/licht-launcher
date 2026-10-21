use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::{db, settings};
use crate::{CoreError, Result, SharedCache};

mod database;
mod files;
mod installation;
mod migration;

pub async fn initialize_launcher() -> Result<SharedCache> {
    let home = SharedCache::system()?;
    let legacy = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
        .ok_or(CoreError::DataDirectory)?;
    initialize_at(&SharedCache::at(legacy.data_dir()), &home).await?;
    Ok(home)
}

async fn initialize_at(legacy: &SharedCache, home: &SharedCache) -> Result<()> {
    let _lock = initialization_lock(home, "layout.lock").await?;

    if !db::db_path(home).exists()
        && (legacy.root().join("settings.json").exists()
            || legacy.root().join("licht.sqlite").exists()
            || legacy.root().join("versions").exists())
    {
        migration::migrate(legacy, home).await?;
    }
    let game = settings::game_cache(home)?;
    tokio::fs::create_dir_all(game.root()).await?;
    tokio::fs::create_dir_all(game.cache_dir()?).await?;
    tokio::fs::create_dir_all(game.instances_dir()?).await?;
    if !home.root().join("settings.json").exists() {
        settings::save_settings(home, &settings::load_settings(home)?)?;
    }
    installation::migrate(home, &game).await?;
    Ok(())
}

pub(crate) async fn initialization_lock(home: &SharedCache, name: &str) -> Result<File> {
    let cache = home.root().join("cache");
    tokio::fs::create_dir_all(&cache).await?;
    let lock_path = cache.join(name);
    tokio::task::spawn_blocking(move || -> Result<File> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.lock()?;
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
