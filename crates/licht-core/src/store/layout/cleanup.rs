use std::fs;
use std::path::Path;

use super::migration::{self, LegacyLayout};
use super::progress::{StartupPhase, StartupReporter};
use super::{Relocation, files};
use crate::store::db::{self, Database};
use crate::{Result, SharedCache};

// The migration rewrote these into the new profile instead of copying them, so they are
// removed last. While they remain, a later startup retries the cleanup.
const PROFILE_FILES: &[&str] = &[
    "licht.sqlite-wal",
    "licht.sqlite-shm",
    "licht.sqlite",
    "settings.json",
];

/// Whether a migrated legacy profile still holds files to remove.
pub(super) fn pending(legacy: &SharedCache, home: &SharedCache) -> bool {
    db::db_path(home).is_file()
        && ["settings.json", "licht.sqlite"]
            .iter()
            .any(|name| legacy.root().join(name).is_file())
}

/// Removes each legacy file whose verified copy is in the new layout, then the profile
/// files and the folders this leaves empty. Files that differ from their copy, or were
/// never copied, stay where they are.
pub(super) async fn remove_legacy(
    legacy: &SharedCache,
    database: &Database,
    reporter: &StartupReporter,
) -> Result<()> {
    reporter.phase(StartupPhase::CleaningLegacyData)?;
    tracing::info!("Removing the migrated legacy profile");
    let LegacyLayout {
        scopes, mappings, ..
    } = migration::legacy_layout(legacy, database.cache()).await?;
    let active = crate::settings::game_cache(database).await?;
    let mut kept = vec![
        database.cache().root().to_path_buf(),
        active.root().to_path_buf(),
        active.instances_dir()?,
        active.cache_dir()?,
    ];
    kept.extend(mappings.iter().map(|mapping| mapping.destination.clone()));

    let mut trees = Vec::new();
    for (source, destination) in &scopes {
        trees.extend(migration::copied_trees(source, destination)?);
        trees.push((source.instances_dir()?, destination.instances_dir()?));
    }
    // A destination inside a legacy folder would make its removal delete live content.
    if trees
        .iter()
        .map(|(source, _)| source.as_path())
        .chain(std::iter::once(legacy.root()))
        .any(|source| {
            kept.iter()
                .any(|path| path.starts_with(source) || source.starts_with(path))
        })
    {
        return Err(std::io::Error::other(
            "the legacy profile overlaps the current launcher directories",
        )
        .into());
    }

    for (source, destination) in &scopes {
        let instances = source.instances_dir()?;
        for (from, to) in migration::copied_trees(source, destination)? {
            remove_copied(&from, &to, &instances, &mappings, reporter)?;
        }
        remove_copied(
            &instances,
            &destination.instances_dir()?,
            &instances,
            &mappings,
            reporter,
        )?;
    }
    for name in PROFILE_FILES {
        match fs::remove_file(legacy.root().join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    // On Windows the legacy profile is `Licht\Licht Launcher\data`. Elsewhere its parent
    // is a shared system folder, which the names below never match.
    if remove_if_empty(legacy.root())?
        && let Some(launcher) = legacy.root().parent().filter(|path| {
            path.file_name()
                .is_some_and(|name| name == "Licht Launcher")
        })
        && remove_if_empty(launcher)?
        && let Some(licht) = launcher
            .parent()
            .filter(|path| path.file_name().is_some_and(|name| name == "Licht"))
    {
        remove_if_empty(licht)?;
    }
    tracing::info!("Migrated legacy profile removed");
    Ok(())
}

/// Removes every file under `source` whose verified copy is at the same place under
/// `destination`, and every folder this empties. Returns whether `source` is gone.
fn remove_copied(
    source: &Path,
    destination: &Path,
    instances: &Path,
    mappings: &[Relocation],
    reporter: &StartupReporter,
) -> Result<bool> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error.into()),
    };
    reporter.checkpoint()?;
    if metadata.is_dir() {
        let mut emptied = true;
        for item in fs::read_dir(source)? {
            let item = item?;
            emptied &= remove_copied(
                &item.path(),
                &destination.join(item.file_name()),
                instances,
                mappings,
                reporter,
            )?;
        }
        if emptied {
            fs::remove_dir(source)?;
        }
        return Ok(emptied);
    }
    let copied = metadata.is_file()
        && fs::symlink_metadata(destination).is_ok_and(|copy| copy.is_file())
        && files::is_verified_copy(source, destination, instances, mappings, reporter)?;
    if copied {
        files::remove_copied_source(source, destination, &metadata)?;
        reporter.file(metadata.len())?;
    }
    Ok(copied)
}

/// Removes `path` when it is an empty folder. Returns whether it is gone.
fn remove_if_empty(path: &Path) -> Result<bool> {
    match fs::read_dir(path) {
        Ok(mut items) => {
            if items.next().is_some() {
                return Ok(false);
            }
            fs::remove_dir(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}
