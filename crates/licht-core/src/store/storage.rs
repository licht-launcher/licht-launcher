use std::path::{Path, PathBuf};

use serde::Serialize;
use sysinfo::{DiskRefreshKind, Disks};
use ts_rs::TS;

use crate::{CoreError, Result, SharedCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StorageCategory {
    Instances,
    GameFiles,
    Natives,
    Java,
    Cache,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub category: StorageCategory,
    #[ts(type = "number")]
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    pub disk_name: String,
    #[ts(type = "number")]
    pub managed_bytes: u64,
    pub categories: Vec<StorageUsage>,
}

/// Logical size of managed files. Links are excluded to avoid traversing external data.
pub fn storage_summary(game: &SharedCache) -> Result<StorageSummary> {
    let directory = std::fs::canonicalize(game.root())?;
    if !directory.is_dir() {
        return Err(CoreError::StorageUnavailable);
    }
    let disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::nothing());
    let volumes = disks.list().iter().filter_map(|disk| {
        // Canonicalize both sides to handle symlinks and Windows verbatim prefixes.
        match std::fs::canonicalize(disk.mount_point()) {
            Ok(mount) => Some((
                mount,
                disk_label(&disk.name().to_string_lossy(), disk.mount_point()),
            )),
            Err(error) => {
                tracing::debug!(%error, "Skipping an inaccessible storage mount");
                None
            }
        }
    });
    let categories = measure_categories(game, &directory)?;
    let managed_bytes = categories.iter().try_fold(0_u64, |total, usage| {
        total
            .checked_add(usage.bytes)
            .ok_or(CoreError::StorageUnavailable)
    })?;
    Ok(StorageSummary {
        disk_name: select_disk(&directory, volumes)?,
        managed_bytes,
        categories,
    })
}

fn measure_categories(game: &SharedCache, directory: &Path) -> Result<Vec<StorageUsage>> {
    let mut game_files = 0_u64;
    let mut natives = 0_u64;
    let mut java = 0_u64;
    let mut other = 0_u64;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let target = match entry.file_name().to_str() {
            Some("runtime") => &mut java,
            Some("assets" | "libraries" | "versions") => &mut game_files,
            Some("natives") => &mut natives,
            _ => &mut other,
        };
        *target = target
            .checked_add(managed_size(&entry.path())?)
            .ok_or(CoreError::StorageUnavailable)?;
    }
    Ok(vec![
        StorageUsage {
            category: StorageCategory::Instances,
            bytes: managed_size(&game.instances_dir()?)?,
        },
        StorageUsage {
            category: StorageCategory::GameFiles,
            bytes: game_files,
        },
        StorageUsage {
            category: StorageCategory::Natives,
            bytes: natives,
        },
        StorageUsage {
            category: StorageCategory::Java,
            bytes: java,
        },
        StorageUsage {
            category: StorageCategory::Cache,
            bytes: managed_size(&game.cache_dir()?)?,
        },
        StorageUsage {
            category: StorageCategory::Other,
            bytes: other,
        },
    ])
}

fn select_disk(
    directory: &Path,
    volumes: impl Iterator<Item = (PathBuf, String)>,
) -> Result<String> {
    volumes
        .filter(|(mount, _)| directory.starts_with(mount))
        .max_by_key(|(mount, _)| mount.components().count())
        .map(|(_, name)| name)
        .ok_or(CoreError::StorageUnavailable)
}

fn disk_label(name: &str, mount: &Path) -> String {
    let mount = mount.to_string_lossy();
    let location = if let Some(network) = mount.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{network}")
    } else {
        mount.strip_prefix(r"\\?\").unwrap_or(&mount).to_string()
    };
    let location = if location == "/" {
        "/"
    } else {
        location.trim_end_matches(['\\', '/'])
    };
    let name = if name.trim().is_empty() {
        "Local Disk"
    } else {
        name.trim()
    };
    format!("{name} ({location})")
}

fn managed_size(directory: &Path) -> Result<u64> {
    let metadata = match std::fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        // Optional managed folders may not exist yet or may disappear during a scan.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    let mut pending = vec![directory.to_path_buf()];
    let mut total = 0_u64;
    while let Some(directory) = pending.pop() {
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Installation and cancellation can remove entries while measuring.
                tracing::debug!("A managed directory disappeared during storage measurement");
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let path = entry?.path();
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tracing::debug!("A managed file disappeared during storage measurement");
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or(CoreError::StorageUnavailable)?;
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_mounts_select_the_volume_containing_the_game_directory() {
        let root = std::env::temp_dir();
        let mounted = root.join("mounted");
        let directory = mounted.join("game");
        let name = select_disk(
            &directory,
            [(mounted, "Game Disk".into()), (root, "Other Disk".into())].into_iter(),
        )
        .expect("mounted volume");
        assert_eq!(name, "Game Disk");
    }

    #[test]
    fn similar_directory_names_do_not_match_an_unrelated_mount() {
        let root = std::env::temp_dir();
        assert!(matches!(
            select_disk(
                &root.join("mounted-other").join("game"),
                [(root.join("mounted"), "Other Disk".into())].into_iter(),
            ),
            Err(CoreError::StorageUnavailable)
        ));
    }

    #[test]
    fn disk_labels_remove_windows_verbatim_prefixes_and_keep_the_drive() {
        assert_eq!(disk_label("", Path::new(r"\\?\C:\")), "Local Disk (C:)");
        assert_eq!(disk_label("Games", Path::new(r"D:\")), "Games (D:)");
        assert_eq!(
            disk_label("Network", Path::new(r"\\?\UNC\server\share\")),
            r"Network (\\server\share)"
        );
        assert_eq!(disk_label("/dev/sda1", Path::new("/")), "/dev/sda1 (/)");
    }

    #[test]
    fn natives_have_their_own_category_without_double_counting() {
        let launcher = fixture_directory();
        let directory = launcher.join("data");
        for (folder, bytes) in [("natives", 19), ("assets", 11)] {
            let folder = directory.join(folder);
            std::fs::create_dir_all(&folder).expect("managed folder");
            std::fs::write(folder.join("payload"), vec![0; bytes]).expect("managed file");
        }
        let summary = storage_summary(&SharedCache::at(&directory)).expect("storage");
        std::fs::remove_dir_all(launcher).expect("cleanup");
        let serialized = serde_json::to_value(&summary).expect("storage json");
        let categories = serialized["categories"].as_array().expect("categories");
        assert!(
            categories
                .iter()
                .any(|usage| usage["category"] == "natives" && usage["bytes"] == 19)
        );
        assert_eq!(summary.managed_bytes, 30);
        assert_eq!(
            summary
                .categories
                .iter()
                .find(|usage| usage.category == StorageCategory::GameFiles)
                .expect("game files")
                .bytes,
            11
        );
    }

    #[test]
    fn managed_consumption_includes_sibling_instances_and_cache() {
        let launcher = fixture_directory();
        for (relative, bytes) in [
            ("instances/first", 3),
            ("cache/icons", 5),
            ("data/runtime", 7),
            ("data/assets", 11),
            ("data/libraries", 13),
            ("data/versions", 17),
            ("data/natives", 19),
            ("data/other", 23),
        ] {
            let folder = launcher.join(relative);
            std::fs::create_dir_all(&folder).expect("managed folder");
            std::fs::write(folder.join("payload"), vec![0; bytes]).expect("managed file");
        }
        std::fs::write(launcher.join("settings.json"), b"not game data").expect("settings");
        let summary = storage_summary(&SharedCache::at(launcher.join("data"))).expect("storage");
        std::fs::remove_dir_all(launcher).expect("cleanup");
        assert_eq!(summary.managed_bytes, 98);
        assert_eq!(
            summary.categories,
            vec![
                StorageUsage {
                    category: StorageCategory::Instances,
                    bytes: 3
                },
                StorageUsage {
                    category: StorageCategory::GameFiles,
                    bytes: 41
                },
                StorageUsage {
                    category: StorageCategory::Natives,
                    bytes: 19
                },
                StorageUsage {
                    category: StorageCategory::Java,
                    bytes: 7
                },
                StorageUsage {
                    category: StorageCategory::Cache,
                    bytes: 5
                },
                StorageUsage {
                    category: StorageCategory::Other,
                    bytes: 23
                },
            ]
        );
    }

    #[test]
    fn managed_files_are_measured_on_their_actual_volume() {
        let launcher = fixture_directory();
        let directory = launcher.join("data");
        for (folder, bytes) in [("instances", 3), ("cache", 5), ("data/runtime", 7)] {
            let folder = launcher.join(folder);
            std::fs::create_dir_all(&folder).expect("managed folder");
            std::fs::write(folder.join("payload"), vec![0; bytes]).expect("managed file");
        }
        let game = SharedCache::at(&directory);
        let summary = storage_summary(&game).expect("local volume");
        assert_eq!(summary.managed_bytes, 15);
        assert!(!summary.disk_name.is_empty());
        assert!(!summary.disk_name.contains(r"\\?\"));
        std::fs::remove_dir_all(launcher).expect("cleanup");
    }

    #[test]
    fn an_empty_managed_directory_has_zero_consumption() {
        let launcher = fixture_directory();
        let directory = launcher.join("data");
        std::fs::create_dir(&directory).expect("data directory");
        let summary = storage_summary(&SharedCache::at(&directory)).expect("empty directory");
        assert_eq!(summary.managed_bytes, 0);
        assert_eq!(summary.categories.len(), 6);
        assert!(summary.categories.iter().all(|usage| usage.bytes == 0));
        std::fs::remove_dir_all(launcher).expect("cleanup");
    }

    #[test]
    fn loose_data_files_are_counted_as_other_and_match_the_category_sum() {
        let launcher = fixture_directory();
        let directory = launcher.join("data");
        std::fs::create_dir(&directory).expect("data directory");
        std::fs::write(directory.join("metadata.json"), b"metadata").expect("metadata");
        let summary = storage_summary(&SharedCache::at(&directory)).expect("storage");
        assert_eq!(summary.managed_bytes, 8);
        assert_eq!(
            summary
                .categories
                .iter()
                .map(|usage| usage.bytes)
                .sum::<u64>(),
            8
        );
        assert_eq!(
            summary.categories.last().expect("other").category,
            StorageCategory::Other
        );
        assert_eq!(summary.categories.last().expect("other").bytes, 8);
        std::fs::remove_dir_all(launcher).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn links_do_not_count_external_files_or_create_directory_cycles() {
        let directory = fixture_directory();
        let outside = fixture_directory();
        std::fs::write(outside.join("external"), b"outside").expect("external file");
        std::fs::write(directory.join("managed"), b"data").expect("managed file");
        std::os::unix::fs::symlink(&outside, directory.join("linked")).expect("external link");
        std::os::unix::fs::symlink(&directory, directory.join("cycle")).expect("cycle");
        assert_eq!(managed_size(&directory).expect("managed size"), 4);
        std::fs::remove_dir_all(directory).expect("cleanup");
        std::fs::remove_dir_all(outside).expect("cleanup external");
    }

    fn fixture_directory() -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("licht-storage-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&directory).expect("fixture directory");
        directory
    }

    #[test]
    fn a_file_cannot_be_used_as_the_game_data_directory() {
        let game = SharedCache::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
        assert!(matches!(
            storage_summary(&game),
            Err(CoreError::StorageUnavailable)
        ));
    }
}
