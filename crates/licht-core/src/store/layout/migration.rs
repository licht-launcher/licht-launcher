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

    for (index, (source, destination)) in [(legacy, &default_game), (&old_game, &new_game)]
        .into_iter()
        .enumerate()
    {
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
        let staged_instances = staging.root().join(format!("instances-{index}"));
        files::copy_tree(&source.instances_dir()?, &staged_instances)?;
        remap_instance_java(&staged_instances, &mappings)?;
        files::copy_tree(&staged_instances, &destination.instances_dir()?)?;
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
    Ok(())
}

fn remap_instance_java(instances: &Path, mappings: &[Relocation]) -> Result<()> {
    if !instances.exists() {
        return Ok(());
    }
    for item in fs::read_dir(instances)? {
        let item = item?;
        if !item.file_type()?.is_dir() {
            continue;
        }
        let file = item.path().join("instance.json");
        if !file.is_file() {
            continue;
        }
        let mut instance: serde_json::Value =
            serde_json::from_slice(&fs::read(&file)?).map_err(CoreError::Instance)?;
        if let Some(path) = instance.get("javaPath").and_then(serde_json::Value::as_str) {
            let path = relocate(Path::new(path), mappings);
            instance["javaPath"] = serde_json::Value::String(path.to_string_lossy().into_owned());
            fs::write(
                file,
                serde_json::to_vec_pretty(&instance).map_err(CoreError::Instance)?,
            )?;
        }
    }
    Ok(())
}
