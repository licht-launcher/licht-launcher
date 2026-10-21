use std::fs;
use std::path::Path;

use super::{Relocation, database, files, relocate};
use crate::{CoreError, Result, SharedCache, load_settings, save_settings};

const GAME_DIRECTORIES: &[&str] = &[
    "assets",
    "libraries",
    "natives",
    "runtime",
    "versions",
    "modpacks",
];

pub(super) async fn migrate(legacy: &SharedCache, home: &SharedCache) -> Result<()> {
    tracing::info!(
        phase = "database-snapshot",
        "Starting launcher storage migration"
    );
    let mut settings = load_settings(legacy)?;
    let old_game = match &settings.data_directory {
        Some(parent) => SharedCache::at(Path::new(parent).join("Licht Launcher")),
        None => legacy.clone(),
    };
    settings = super::super::settings::validated(home, settings)?;
    let new_game = match &settings.data_directory {
        Some(parent) => SharedCache::at(Path::new(parent).join("Licht Launcher").join("data")),
        None => SharedCache::at(home.root().join("data")),
    };
    let default_game = SharedCache::at(home.root().join("data"));
    let mut mappings = vec![
        Relocation {
            source: old_game.root().to_path_buf(),
            destination: new_game.root().to_path_buf(),
        },
        Relocation {
            source: old_game.instances_dir()?,
            destination: new_game.instances_dir()?,
        },
        Relocation {
            source: legacy.root().to_path_buf(),
            destination: default_game.root().to_path_buf(),
        },
        Relocation {
            source: legacy.instances_dir()?,
            destination: default_game.instances_dir()?,
        },
    ];
    mappings.sort_by_key(|mapping| std::cmp::Reverse(mapping.source.components().count()));
    mappings.dedup_by(|left, right| {
        left.source == right.source && left.destination == right.destination
    });

    let staging = SharedCache::at(home.root().join("cache").join("layout-migration"));
    if staging.root().exists() {
        fs::remove_dir_all(staging.root())?;
    }
    fs::create_dir_all(staging.root())?;
    database::snapshot(&legacy.root().join("licht.sqlite"), &staging).await?;
    database::remap(&staging, &mappings).await?;

    let mut scopes = vec![(legacy, &default_game)];
    if old_game != *legacy || new_game != default_game {
        scopes.push((&old_game, &new_game));
    }
    for (index, (source, destination)) in scopes.into_iter().enumerate() {
        tracing::info!(
            phase = "shared-files",
            scope = index + 1,
            "Migrating launcher storage"
        );
        for directory in GAME_DIRECTORIES {
            files::copy_tree(
                &source.root().join(directory),
                &destination.root().join(directory),
            )?;
        }
        files::copy_tree(
            &source.root().join("content-metadata"),
            &destination.cache_dir()?.join("content-metadata"),
        )?;
        tracing::info!(
            phase = "instances",
            scope = index + 1,
            "Migrating launcher storage"
        );
        copy_instances(
            &source.instances_dir()?,
            &destination.instances_dir()?,
            &staging.root().join(format!("profiles-{index}")),
            &mappings,
        )?;
    }
    for path in [
        &mut settings.java25,
        &mut settings.java21,
        &mut settings.java17,
        &mut settings.java8,
    ]
    .into_iter()
    .flatten()
    {
        *path = relocate(Path::new(path), &mappings)
            .to_string_lossy()
            .into_owned();
    }
    tracing::info!(phase = "settings", "Migrating launcher storage");
    let staged_settings = save_settings(&staging, &settings)?;
    let settings_file = home.root().join("settings.json");
    if settings_file.exists() && settings_file != legacy.root().join("settings.json") {
        if load_settings(home)? != staged_settings {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "launcher migration settings destination differs",
            )
            .into());
        }
    } else {
        save_settings(home, &staged_settings)?;
    }
    // Publishing the validated database last marks a completed migration.
    fs::rename(super::db::db_path(&staging), super::db::db_path(home))?;
    fs::remove_dir_all(staging.root())?;
    tracing::info!("Launcher storage migration completed");
    Ok(())
}

fn copy_instances(
    source: &Path,
    destination: &Path,
    staging: &Path,
    mappings: &[Relocation],
) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for item in fs::read_dir(source)? {
        let item = item?;
        let target = destination.join(item.file_name());
        if !item.file_type()?.is_dir() {
            files::copy_tree(&item.path(), &target)?;
            continue;
        }
        if target.exists() && fs::symlink_metadata(&target)?.file_type().is_symlink() {
            return Err(CoreError::CachePath);
        }
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(item.path())? {
            let entry = entry?;
            let source = entry.path();
            let destination = target.join(entry.file_name());
            if entry.file_name() != "instance.json" {
                files::copy_tree(&source, &destination)?;
                continue;
            }
            if !entry.file_type()?.is_file() {
                return Err(CoreError::CachePath);
            }
            let mut instance: serde_json::Value =
                serde_json::from_slice(&fs::read(&source)?).map_err(CoreError::Instance)?;
            if let Some(path) = instance.get("javaPath").and_then(serde_json::Value::as_str) {
                let path = relocate(Path::new(path), mappings);
                instance["javaPath"] =
                    serde_json::Value::String(path.to_string_lossy().into_owned());
                let staged = staging.join(item.file_name()).join("instance.json");
                fs::create_dir_all(staged.parent().ok_or(CoreError::CachePath)?)?;
                fs::write(
                    &staged,
                    serde_json::to_vec_pretty(&instance).map_err(CoreError::Instance)?,
                )?;
                files::copy_file(&staged, &destination)?;
            } else {
                files::copy_file(&source, &destination)?;
            }
        }
    }
    Ok(())
}
