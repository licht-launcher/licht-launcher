use std::fs;
use std::path::{Path, PathBuf};

use super::metadata::fallback_title;
use super::{InstanceContentEntry, InstanceContentKind};
use crate::{CoreError, Result, SharedCache};

pub(super) const DISABLED_SUFFIX: &str = ".disabled";

pub fn list_instance_content(
    cache: &SharedCache,
    folder: &str,
) -> Result<Vec<InstanceContentEntry>> {
    let root = instance_root(cache, folder)?;
    let mut entries = Vec::new();
    for (directory, kind) in [
        ("mods", InstanceContentKind::Mods),
        ("resourcepacks", InstanceContentKind::ResourcePacks),
        ("shaderpacks", InstanceContentKind::ShaderPacks),
    ] {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        for item in fs::read_dir(path)? {
            let item = item?;
            let metadata = item.metadata()?;
            if !metadata.is_file() {
                continue;
            }
            let raw_name = item.file_name().to_string_lossy().into_owned();
            let enabled = !raw_name.ends_with(DISABLED_SUFFIX);
            let name = raw_name
                .strip_suffix(DISABLED_SUFFIX)
                .unwrap_or(&raw_name)
                .to_string();
            if kind == InstanceContentKind::ShaderPacks
                && !name.to_ascii_lowercase().ends_with(".zip")
            {
                continue;
            }
            entries.push(InstanceContentEntry {
                title: fallback_title(&name),
                name,
                kind,
                enabled,
                size: metadata.len(),
                version: None,
                icon_url: None,
            });
        }
    }
    entries.sort_by(|left, right| {
        left.kind
            .sort_key()
            .cmp(&right.kind.sort_key())
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
    });
    Ok(entries)
}

pub fn set_instance_content_enabled(
    cache: &SharedCache,
    folder: &str,
    kind: InstanceContentKind,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let root = instance_root(cache, folder)?;
    let current_name = if enabled {
        format!("{name}{DISABLED_SUFFIX}")
    } else {
        name.to_string()
    };
    let next_name = if enabled {
        name.to_string()
    } else {
        format!("{name}{DISABLED_SUFFIX}")
    };
    let current = content_file(&root, content_directory(kind), &current_name)?;
    let next = content_file(&root, content_directory(kind), &next_name)?;
    if !current.is_file() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "content file is missing",
        )));
    }
    if next.exists() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "content file already exists",
        )));
    }
    fs::rename(current, next)?;
    Ok(())
}

pub fn remove_instance_content(
    cache: &SharedCache,
    folder: &str,
    kind: InstanceContentKind,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let root = instance_root(cache, folder)?;
    let filename = if enabled {
        name.to_string()
    } else {
        format!("{name}{DISABLED_SUFFIX}")
    };
    let path = content_file(&root, content_directory(kind), &filename)?;
    if !path.is_file() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "content file is missing",
        )));
    }
    fs::remove_file(path)?;
    Ok(())
}

pub(super) fn instance_root(cache: &SharedCache, folder: &str) -> Result<PathBuf> {
    let root = cache.instance_dir(folder)?;
    if !root.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    Ok(root)
}

pub(super) fn content_file(root: &Path, directory: &str, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
        || name.contains(':')
    {
        return Err(CoreError::CachePath);
    }
    Ok(root.join(directory).join(name))
}

pub(super) fn content_directory(kind: InstanceContentKind) -> &'static str {
    match kind {
        InstanceContentKind::Mods => "mods",
        InstanceContentKind::ResourcePacks => "resourcepacks",
        InstanceContentKind::ShaderPacks => "shaderpacks",
    }
}
