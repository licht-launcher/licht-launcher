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
        .map(|path| enabled_path(&path.replace('\\', "/")))
        .collect())
}

/// `path` with the `.disabled` suffix taken off the content it names, so a file the
/// pack ships disabled, or a file inside a folder pack it ships disabled, matches the
/// entry listed for it.
fn enabled_path(path: &str) -> String {
    let mut parts = path.splitn(3, '/');
    let directory = parts.next().unwrap_or_default();
    let Some(name) = parts.next() else {
        return path.to_string();
    };
    let name = name.strip_suffix(DISABLED_SUFFIX).unwrap_or(name);
    match parts.next() {
        Some(rest) => format!("{directory}/{name}/{rest}"),
        None => format!("{directory}/{name}"),
    }
}

/// Enabling or disabling a modpack file only renames it, so it still belongs to the pack.
/// A shader pack kept as a folder belongs to it when the pack copied files into it.
pub(super) fn mark_modpack_content(entries: &mut [InstanceContentEntry], paths: &BTreeSet<String>) {
    for entry in entries {
        let relative = format!("{}/{}", content_directory(entry.kind), entry.name);
        let folder = format!("{relative}/");
        entry.from_modpack = paths.contains(&relative)
            || paths
                .range(folder.clone()..)
                .next()
                .is_some_and(|path| path.starts_with(&folder));
    }
}
