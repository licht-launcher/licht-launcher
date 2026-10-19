use super::{
    GlobalLaunch, InstanceEntry, InstanceSettings, copy_tree, directory_name, display_name,
    entry_from, folder_id, instance_dir, instance_from_template, load_instance,
    normalize_java_path, profile_guard, single_component, utc_now_rfc3339, validate,
    write_instance,
};
use crate::{CoreError, Result};
use std::fs;
use std::path::Path;

/// Loads every direct child directory. A missing `instances` directory is an empty list.
pub fn list_instances(instances_dir: &Path) -> Result<Vec<InstanceEntry>> {
    let _guard = profile_guard()?;
    if !instances_dir.exists() {
        return Ok(Vec::new());
    }
    let mut instances = Vec::new();
    for entry in fs::read_dir(instances_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            let folder = directory_name(&path)?;
            let instance = load_instance(&path)?;
            instances.push(entry_from(&path, folder, instance));
        }
    }
    instances.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.folder.cmp(&right.folder))
    });
    Ok(instances)
}
pub fn create_instance(
    instances_dir: &Path,
    name: &str,
    version_id: &str,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let name = display_name(name)?;
    single_component(version_id)?;
    let folder = folder_id(&name)?;
    fs::create_dir_all(instances_dir)?;
    let dir = instances_dir.join(&folder);
    if dir.exists() {
        return Err(CoreError::InstanceExists);
    }
    fs::create_dir(&dir)?;
    let instance = instance_from_template(name, version_id.to_string(), template);
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder, instance))
}
pub fn rename_instance(instances_dir: &Path, folder: &str, name: &str) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let name = display_name(name)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    instance.name = name;
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
pub fn duplicate_instance(instances_dir: &Path, folder: &str, name: &str) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let name = display_name(name)?;
    let source = instance_dir(instances_dir, folder)?;
    let _ = load_instance(&source)?;
    let new_folder = folder_id(&name)?;
    let destination = instances_dir.join(&new_folder);
    if destination.exists() {
        return Err(CoreError::InstanceExists);
    }
    if let Err(error) = copy_tree(&source, &destination) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }
    let mut instance = load_instance(&destination)?;
    instance.name = name;
    instance.last_opened_at = None;
    if let Err(error) = write_instance(&destination, &instance) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }
    Ok(entry_from(&destination, new_folder, instance))
}
/// Records that the instance was opened now (RFC 3339 UTC on the profile).
pub fn touch_last_opened(instances_dir: &Path, folder: &str) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    instance.last_opened_at = Some(utc_now_rfc3339()?);
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
pub fn delete_instance(instances_dir: &Path, folder: &str) -> Result<()> {
    let dir = instance_dir(instances_dir, folder)?;
    fs::remove_dir_all(dir)?;
    Ok(())
}
/// Writes launch personalization. Name and version stay as stored on disk.
pub fn update_instance(
    instances_dir: &Path,
    folder: &str,
    settings: &InstanceSettings,
) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    instance.override_window = settings.override_window;
    instance.override_memory = settings.override_memory;
    instance.override_java = settings.override_java;
    instance.override_jvm_arguments = settings.override_jvm_arguments;
    instance.java_path = normalize_java_path(settings.java_path.as_deref());
    instance.max_memory_mb = settings.max_memory_mb;
    instance.jvm_arguments = settings.jvm_arguments.clone();
    instance.fullscreen = settings.fullscreen;
    instance.width = settings.width;
    instance.height = settings.height;
    validate(&instance)?;
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
/// Points the instance at another game version. Does not download files.
pub fn update_instance_version(
    instances_dir: &Path,
    folder: &str,
    version_id: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    single_component(version_id)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    instance.version_id = Some(version_id.to_string());
    validate(&instance)?;
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
