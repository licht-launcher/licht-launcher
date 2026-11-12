use std::fs;
use std::path::{Path, PathBuf};

use super::progress::{StartupPhase, StartupReporter};
use super::{Relocation, database, files, relocate};
use crate::settings::{Settings, load_settings_at, store_settings};
use crate::store::db::Database;
use crate::{CoreError, Result, SharedCache};

const GAME_DIRECTORIES: &[&str] = &[
    "assets",
    "libraries",
    "natives",
    "runtime",
    "versions",
    "modpacks",
];

/// Where the content of a legacy profile goes in the new layout.
pub(super) struct LegacyLayout {
    /// The legacy settings, validated for the new profile.
    pub(super) settings: Settings,
    /// Each copied game directory with its destination, the legacy profile first.
    pub(super) scopes: Vec<(SharedCache, SharedCache)>,
    pub(super) mappings: Vec<Relocation>,
}

pub(super) async fn legacy_layout(
    legacy: &SharedCache,
    home: &SharedCache,
) -> Result<LegacyLayout> {
    let settings = load_settings_at(legacy).await?;
    let old_game = match &settings.data_directory {
        Some(parent) => SharedCache::at(Path::new(parent).join("Licht Launcher")),
        None => legacy.clone(),
    };
    let settings = super::super::settings::validated(home, settings, crate::total_memory_mb())?;
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
    let mut scopes = vec![(legacy.clone(), default_game.clone())];
    if old_game != *legacy || new_game != default_game {
        scopes.push((old_game, new_game));
    }
    Ok(LegacyLayout {
        settings,
        scopes,
        mappings,
    })
}

/// The directories copied unchanged from one scope, paired with their destinations.
/// Instances are copied separately because their profiles are rewritten.
pub(super) fn copied_trees(
    source: &SharedCache,
    destination: &SharedCache,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut trees = GAME_DIRECTORIES
        .iter()
        .map(|directory| {
            (
                source.root().join(directory),
                destination.root().join(directory),
            )
        })
        .collect::<Vec<_>>();
    trees.push((
        source.root().join("content-metadata"),
        destination.cache_dir()?.join("content-metadata"),
    ));
    Ok(trees)
}

pub(super) async fn migrate(
    legacy: &SharedCache,
    home: &SharedCache,
    reporter: &StartupReporter,
) -> Result<()> {
    reporter.phase(StartupPhase::MigratingDatabase)?;
    tracing::info!(
        phase = "database-snapshot",
        "Starting launcher storage migration"
    );
    let LegacyLayout {
        mut settings,
        scopes,
        mappings,
    } = legacy_layout(legacy, home).await?;

    let staging = SharedCache::at(home.root().join("cache").join("layout-migration"));
    if staging.root().exists() {
        fs::remove_dir_all(staging.root())?;
    }
    fs::create_dir_all(staging.root())?;
    database::snapshot(&legacy.root().join("licht.sqlite"), &staging).await?;
    // The pool closes before the file copies, so a failed copy leaves nothing open.
    let staged_database = Database::open(&staging).await?;
    let remapped = database::remap(&staged_database, &mappings).await;
    staged_database.close().await;
    remapped?;

    for (index, (source, destination)) in scopes.iter().enumerate() {
        reporter.phase(StartupPhase::MigratingData)?;
        tracing::info!(
            phase = "shared-files",
            scope = index + 1,
            "Migrating launcher storage"
        );
        for (from, to) in copied_trees(source, destination)? {
            files::copy_tree_with_progress(&from, &to, reporter)?;
        }
        tracing::info!(
            phase = "instances",
            scope = index + 1,
            "Migrating launcher storage"
        );
        reporter.phase(StartupPhase::MigratingInstances)?;
        copy_instances(
            &source.instances_dir()?,
            &destination.instances_dir()?,
            &staging.root().join(format!("profiles-{index}")),
            &mappings,
            reporter,
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
    reporter.phase(StartupPhase::MigratingSettings)?;
    let staged_database = Database::open(&staging).await?;
    let staged = async {
        let saved = store_settings(&staged_database, &settings).await?;
        // Only licht.db is published, so the settings row must not stay in its WAL.
        database::checkpoint(&staged_database).await?;
        Ok::<_, CoreError>(saved)
    }
    .await;
    // The staged file is renamed below, so no connection may stay open on it.
    staged_database.close().await;
    let staged_settings = staged?;
    let settings_file = home.root().join("settings.json");
    if settings_file.exists() && settings_file != legacy.root().join("settings.json") {
        // An interrupted attempt may already have published this same file.
        let published =
            read_json(&settings_file)? == read_json(&staging.root().join("settings.json"))?;
        if !published && load_settings_at(home).await? != staged_settings {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "launcher migration settings destination differs",
            )
            .into());
        }
    } else {
        // Saving into the profile would create its database before the staged one is
        // published, so the staged file is moved into place instead.
        fs::rename(
            staging.root().join("settings.json"),
            home.root().join("settings.json"),
        )?;
    }
    // Publishing the validated database last marks a completed migration.
    fs::rename(super::db::db_path(&staging), super::db::db_path(home))?;
    fs::remove_dir_all(staging.root())?;
    tracing::info!("Launcher storage migration completed");
    Ok(())
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    serde_json::from_slice(&fs::read(path)?).map_err(CoreError::Settings)
}

pub(super) fn copy_instances(
    source: &Path,
    destination: &Path,
    staging: &Path,
    mappings: &[Relocation],
    reporter: &StartupReporter,
) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for item in fs::read_dir(source)? {
        let item = item?;
        let target = destination.join(item.file_name());
        if !item.file_type()?.is_dir() {
            files::copy_tree_with_progress(&item.path(), &target, reporter)?;
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
                files::copy_tree_with_progress(&source, &destination, reporter)?;
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
                files::copy_file_with_progress(&staged, &destination, reporter)?;
            } else {
                files::copy_file_with_progress(&source, &destination, reporter)?;
            }
        }
    }
    Ok(())
}
