use std::collections::BTreeSet;

use super::InstanceContentEntry;
use super::files::{DISABLED_SUFFIX, content_directory};
use crate::{DatabaseClient, Result, SharedCache};

/// Instance-relative paths (`mods/sodium.jar`) that the instance's modpack installed,
/// from its index or its overrides folder. A pack may ship a file disabled, so the
/// `.disabled` suffix is dropped to match listed entries, which never carry it.
pub(super) async fn modpack_paths(
    cache: &SharedCache,
    database: &DatabaseClient,
    folder: &str,
) -> Result<BTreeSet<String>> {
    let instances = cache.instances_dir()?;
    let modrinth = database.modpack_files(&instances, folder).await?;
    let curseforge = database.curseforge_files(&instances, folder).await?;
    let overrides = database.modpack_override_files(&instances, folder).await?;
    Ok(modrinth
        .into_iter()
        .map(|file| file.relative_path)
        .chain(curseforge.into_iter().map(|file| file.relative_path))
        .chain(overrides)
        .map(|path| {
            let path = path.replace('\\', "/");
            path.strip_suffix(DISABLED_SUFFIX)
                .map_or_else(|| path.clone(), str::to_string)
        })
        .collect())
}

/// Enabling or disabling a modpack file only renames it, so it still belongs to the pack.
pub(super) fn mark_modpack_content(entries: &mut [InstanceContentEntry], paths: &BTreeSet<String>) {
    for entry in entries {
        let relative = format!("{}/{}", content_directory(entry.kind), entry.name);
        entry.from_modpack = paths.contains(&relative);
    }
}
