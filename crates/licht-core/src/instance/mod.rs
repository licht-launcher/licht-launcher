//! Instance profiles (`instance.json`) and folder listing.

pub(crate) mod content;
pub mod icon;
pub(crate) mod listing;
mod operations;
mod persistence;
mod preparation;

pub use operations::{
    create_instance, delete_instance, duplicate_instance, list_instances, rename_instance,
    touch_last_opened, update_instance, update_instance_version,
};
pub use persistence::{load_instance, parse_instance};
use persistence::{validate, write_instance};
pub use preparation::{
    InstanceInstallation, InstancePreparationState, InstanceSource, complete_instance_preparation,
    configure_instance_version, create_pending_instance, instance_version_for_launch,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::instance::icon::ICON_FILE;
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result};

const SCHEMA: u32 = 2;
const DEFAULT_MIN_MEMORY_MB: u32 = 512;
const DEFAULT_MAX_MEMORY_MB: u32 = 2048;
const FILE_NAME: &str = "instance.json";
// Background preparation and UI edits must not overwrite each other's profile changes.
static PROFILE_MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn profile_guard() -> Result<std::sync::MutexGuard<'static, ()>> {
    PROFILE_MUTATIONS
        .lock()
        .map_err(|_| CoreError::Io(std::io::Error::other("instance profile lock was poisoned")))
}

/// Profile stored at `instances/<folder>/instance.json`.
///
/// LaunchWrapper keeps the game in a child `.minecraft` folder, so this file
/// stays beside that folder instead of inside the directory the old client opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub schema: u32,
    pub name: String,
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installation: Option<InstanceInstallation>,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub jvm_arguments: Vec<String>,
    #[serde(default)]
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(default)]
    pub override_window: bool,
    #[serde(default)]
    pub override_memory: bool,
    #[serde(default)]
    pub override_java: bool,
    #[serde(default)]
    pub override_jvm_arguments: bool,
    #[serde(default)]
    pub java_path: Option<String>,
    /// Last time this instance was launched (RFC 3339 UTC). Absent if never played.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<String>,
}

/// An instance folder and the profile stored in it.
///
/// `folder` is chosen once and does not change when the profile is renamed.
/// `icon_path` is the absolute path to `icon.png` when present beside `instance.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceEntry {
    pub folder: String,
    pub name: String,
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub installation: Option<InstanceInstallation>,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub jvm_arguments: Vec<String>,
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub override_window: bool,
    pub override_memory: bool,
    pub override_java: bool,
    pub override_jvm_arguments: bool,
    pub java_path: Option<String>,
    pub last_opened_at: Option<String>,
    pub icon_path: Option<String>,
}

/// Launch personalization written from the instance settings dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSettings {
    pub override_window: bool,
    pub override_memory: bool,
    pub override_java: bool,
    pub override_jvm_arguments: bool,
    pub java_path: Option<String>,
    pub max_memory_mb: u32,
    pub jvm_arguments: Vec<String>,
    pub fullscreen: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

fn default_instance(name: String, version_id: String) -> Instance {
    Instance {
        schema: SCHEMA,
        name,
        version_id: Some(version_id),
        installation: None,
        min_memory_mb: DEFAULT_MIN_MEMORY_MB,
        max_memory_mb: DEFAULT_MAX_MEMORY_MB,
        jvm_arguments: Vec::new(),
        fullscreen: false,
        width: None,
        height: None,
        override_window: false,
        override_memory: false,
        override_java: false,
        override_jvm_arguments: false,
        java_path: None,
        last_opened_at: None,
    }
}

fn instance_from_template(name: String, version_id: String, template: &GlobalLaunch) -> Instance {
    Instance {
        schema: SCHEMA,
        name,
        version_id: Some(version_id),
        installation: None,
        min_memory_mb: DEFAULT_MIN_MEMORY_MB,
        max_memory_mb: template.max_memory_mb,
        jvm_arguments: template.jvm_arguments.clone(),
        fullscreen: template.fullscreen,
        width: template.width,
        height: template.height,
        override_window: false,
        override_memory: false,
        override_java: false,
        override_jvm_arguments: false,
        java_path: None,
        last_opened_at: None,
    }
}

/// Launch values stored on this profile. Settings are not read again.
pub fn instance_launch(instance: &Instance) -> GlobalLaunch {
    GlobalLaunch {
        max_memory_mb: instance.max_memory_mb,
        jvm_arguments: instance.jvm_arguments.clone(),
        fullscreen: instance.fullscreen,
        width: instance.width,
        height: instance.height,
    }
}

/// Merges instance overrides with global settings. Global wins where override is off.
pub fn resolved_launch(instance: &Instance, global: &GlobalLaunch) -> GlobalLaunch {
    GlobalLaunch {
        max_memory_mb: if instance.override_memory {
            instance.max_memory_mb
        } else {
            global.max_memory_mb
        },
        jvm_arguments: if instance.override_jvm_arguments {
            instance.jvm_arguments.clone()
        } else {
            global.jvm_arguments.clone()
        },
        fullscreen: if instance.override_window {
            instance.fullscreen
        } else {
            global.fullscreen
        },
        width: if instance.override_window {
            instance.width
        } else {
            global.width
        },
        height: if instance.override_window {
            instance.height
        } else {
            global.height
        },
    }
}

/// Custom Java for this instance when override is on and a path is set.
pub fn resolved_java_path(instance: &Instance) -> Option<PathBuf> {
    if !instance.override_java {
        return None;
    }
    let path = instance.java_path.as_deref()?.trim();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

/// Builds an [`InstanceEntry`] from the profile and optional cached icon.
pub fn instance_entry(dir: &Path, folder: &str) -> Result<InstanceEntry> {
    let instance = load_instance(dir)?;
    Ok(entry_from(dir, folder.to_string(), instance))
}

fn entry_from(dir: &Path, folder: String, instance: Instance) -> InstanceEntry {
    let icon = icon::cached_icon_path(dir, icon::source_icon_url(&instance))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| dir.join(ICON_FILE));
    let icon_path = icon.is_file().then(|| icon.to_string_lossy().into_owned());
    InstanceEntry {
        folder,
        name: instance.name,
        version_id: instance.version_id,
        installation: instance.installation,
        min_memory_mb: instance.min_memory_mb,
        max_memory_mb: instance.max_memory_mb,
        jvm_arguments: instance.jvm_arguments,
        fullscreen: instance.fullscreen,
        width: instance.width,
        height: instance.height,
        override_window: instance.override_window,
        override_memory: instance.override_memory,
        override_java: instance.override_java,
        override_jvm_arguments: instance.override_jvm_arguments,
        java_path: instance.java_path,
        last_opened_at: instance.last_opened_at,
        icon_path,
    }
}

fn utc_now_rfc3339() -> Result<String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| CoreError::Io(std::io::Error::other(error)))?;
    Ok(unix_secs_to_rfc3339(duration.as_secs()))
}

/// Civil UTC date/time from Unix seconds (no leap seconds), as `YYYY-MM-DDTHH:MM:SSZ`.
fn unix_secs_to_rfc3339(secs: u64) -> String {
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let hour = tod / 3_600;
    let min = (tod % 3_600) / 60;
    let sec = tod % 60;
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Howard Hinnant's `civil_from_days` for the proleptic Gregorian calendar.
fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year as i32, month, day)
}

fn instance_dir(instances_dir: &Path, folder: &str) -> Result<PathBuf> {
    single_component(folder)?;
    let dir = instances_dir.join(folder);
    if !dir.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    Ok(dir)
}

fn display_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return Err(CoreError::InstanceName);
    }
    Ok(name.to_string())
}

fn folder_id(name: &str) -> Result<String> {
    let mut folder = String::new();
    let mut hyphen = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '.' || character == '_' {
            if hyphen && !folder.is_empty() {
                folder.push('-');
            }
            hyphen = false;
            folder.push(character);
        } else if character.is_whitespace() || character == '-' {
            hyphen = true;
        }
    }
    single_component(&folder).map_err(|_| CoreError::InstanceName)?;
    Ok(folder)
}

fn copy_tree(source: &Path, destination: &Path, skip_install: bool) -> Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if skip_install
            && entry.file_name().to_str().is_some_and(|name| {
                name.eq_ignore_ascii_case(crate::modpacks::workspace::INSTALL_DIRECTORY)
            })
        {
            continue;
        }
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target, false)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(CoreError::CachePath);
        }
    }
    Ok(())
}

fn normalize_java_path(path: Option<&str>) -> Option<String> {
    let path = path?.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

fn directory_name(dir: &Path) -> Result<String> {
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CoreError::CachePath)?;
    single_component(name)?;
    Ok(name.to_string())
}

fn single_component(name: &str) -> Result<()> {
    use crate::path_component::path_component_safe;

    // Same rules as modpack path segments (reserved names, `:`, trailing junk).
    if name.contains('/') || name.contains('\\') || !path_component_safe(name) {
        return Err(CoreError::CachePath);
    }
    Ok(())
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn rejects_drive_relative_reserved_and_trailing_junk_folder_names() {
        for name in [
            "C:evil", "foo:bar", "a\0b", "..", ".", "a/b", "a\\b", "", "CON", "nul.txt", "lpt1",
            "foo.", "foo ",
        ] {
            assert!(
                matches!(single_component(name), Err(CoreError::CachePath)),
                "{name:?}"
            );
        }
        assert!(single_component("vanilla-1.20").is_ok());
    }
}
