use std::collections::BTreeSet;

use super::InstanceContentEntry;
use super::files::content_directory;
use crate::{DatabaseClient, Result, SharedCache};

/// Instance-relative paths (`mods/sodium.jar`) that the instance's modpack installed.
pub(super) async fn modpack_paths(
    cache: &SharedCache,
    database: &DatabaseClient,
    folder: &str,
) -> Result<BTreeSet<String>> {
    let instances = cache.instances_dir()?;
    let modrinth = database.modpack_files(&instances, folder).await?;
    let curseforge = database.curseforge_files(&instances, folder).await?;
    Ok(modrinth
        .into_iter()
        .map(|file| file.relative_path)
        .chain(curseforge.into_iter().map(|file| file.relative_path))
        .map(|path| path.replace('\\', "/"))
        .collect())
}

/// Disabling a modpack file only renames it, so it still belongs to the pack.
pub(super) fn mark_modpack_content(entries: &mut [InstanceContentEntry], paths: &BTreeSet<String>) {
    for entry in entries {
        let relative = format!("{}/{}", content_directory(entry.kind), entry.name);
        entry.from_modpack = paths.contains(&relative);
    }
}
