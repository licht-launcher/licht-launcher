use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use ts_rs::TS;

use crate::cache::SharedCache;
use crate::{CoreError, Result};

/// One file or directory inside an instance folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceFsEntry {
    pub name: String,
    pub is_dir: bool,
    // JSON numbers. A file size fits in a JavaScript number, unlike `bigint`.
    #[ts(type = "number | null")]
    pub size: Option<u64>,
    /// Unix epoch milliseconds. Birth time when available, else modified time.
    #[ts(type = "number | null")]
    pub created: Option<u64>,
}

/// Lists files and directories under `relative_path` inside an instance.
///
/// `relative_path` is relative to the instance root (empty string = root).
/// The resolved path is canonicalized and must stay inside the instance directory.
pub fn list_instance_entries(
    cache: &SharedCache,
    folder: &str,
    relative_path: &str,
) -> Result<Vec<InstanceFsEntry>> {
    let root = cache.instance_dir(folder)?;
    if !root.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    let dir = resolve_within(&root, relative_path)?;
    if !dir.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "path is not a directory",
        )));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let is_dir = metadata.is_dir();
        entries.push(InstanceFsEntry {
            name,
            is_dir,
            size: if is_dir { None } else { Some(metadata.len()) },
            created: entry_created_ms(&metadata),
        });
    }

    entries.sort_by(|left, right| {
        right.is_dir.cmp(&left.is_dir).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    Ok(entries)
}

fn entry_created_ms(metadata: &fs::Metadata) -> Option<u64> {
    let time = metadata.created().or_else(|_| metadata.modified()).ok()?;
    system_time_ms(time)
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn resolve_within(root: &Path, relative_path: &str) -> Result<PathBuf> {
    let root = fs::canonicalize(root)?;
    let mut candidate = root.clone();
    for component in relative_path.split(['/', '\\']) {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." || component.contains(':') {
            return Err(CoreError::CachePath);
        }
        candidate.push(component);
    }
    let canonical = fs::canonicalize(&candidate)?;
    if !canonical.starts_with(&root) {
        return Err(CoreError::CachePath);
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_cache(name: &str) -> SharedCache {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-instance-fs-{}-{name}-{stamp}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("cache root");
        SharedCache::at(root)
    }

    fn prepare_instance(cache: &SharedCache, folder: &str) -> PathBuf {
        let dir = cache.instance_dir(folder).expect("instance dir");
        fs::create_dir_all(&dir).expect("create instance");
        dir
    }

    #[test]
    fn lists_root_with_directories_first() {
        let cache = temp_cache("list-root");
        let root = prepare_instance(&cache, "classic");
        fs::create_dir_all(root.join("saves")).expect("saves");
        fs::write(root.join("options.txt"), b"fov:70\n").expect("options");
        fs::write(root.join("instance.json"), b"{}").expect("json");

        let entries = list_instance_entries(&cache, "classic", "").expect("list");
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["saves", "instance.json", "options.txt"]);
        assert!(entries[0].is_dir);
        assert!(!entries[1].is_dir);
        assert_eq!(entries[2].size, Some(b"fov:70\n".len() as u64));
        assert!(entries.iter().all(|entry| entry.created.is_some()));
    }

    #[test]
    fn lists_a_subdirectory() {
        let cache = temp_cache("list-sub");
        let root = prepare_instance(&cache, "classic");
        let saves = root.join("saves").join("world");
        fs::create_dir_all(&saves).expect("world");
        fs::write(saves.join("level.dat"), b"data").expect("level");

        let entries = list_instance_entries(&cache, "classic", "saves/world").expect("list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "level.dat");
        assert!(!entries[0].is_dir);
    }

    #[test]
    fn rejects_parent_directory_escape() {
        let cache = temp_cache("escape");
        prepare_instance(&cache, "classic");

        let error = list_instance_entries(&cache, "classic", "..").expect_err("parent escape");
        assert!(matches!(error, CoreError::CachePath));

        let error = list_instance_entries(&cache, "classic", "saves/../../outside")
            .expect_err("nested escape");
        assert!(matches!(error, CoreError::CachePath));
    }

    #[test]
    fn rejects_missing_instance_directory() {
        let cache = temp_cache("missing");
        let error = list_instance_entries(&cache, "gone", "").expect_err("missing");
        assert!(matches!(error, CoreError::Io(_)));
    }
}
