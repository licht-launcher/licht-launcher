use super::{
    GlobalLaunch, InstanceEntry, InstanceSettings, copy_tree, display_name, entry_from, folder_id,
    instance_dir, instance_from_template, load_instance, normalize_java_path, profile_guard,
    single_component, utc_now_rfc3339, validate, write_instance,
};
use crate::{CoreError, Result};
use std::fs;
use std::path::Path;

/// Lists registered profiles whose instance directories still exist.
pub async fn list_instances(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
) -> Result<Vec<InstanceEntry>> {
    let _guard = profile_guard().await;
    if !instances_dir.exists() {
        return Ok(Vec::new());
    }
    let mut instances = Vec::new();
    for (folder, instance) in database.instance_profiles(instances_dir).await? {
        let path = instances_dir.join(&folder);
        if path.is_dir() {
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
pub async fn create_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    name: &str,
    version_id: &str,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
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
    if let Err(error) = write_instance(database, &dir, &instance).await {
        fs::remove_dir(&dir)?;
        return Err(error);
    }
    Ok(entry_from(&dir, folder, instance))
}
pub async fn rename_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    name: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let name = display_name(name)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    instance.name = name;
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
pub async fn duplicate_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    name: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let name = display_name(name)?;
    let source = instance_dir(instances_dir, folder)?;
    let original = load_instance(database, &source).await?;
    let new_folder = folder_id(&name)?;
    let destination = instances_dir.join(&new_folder);
    if destination.exists() {
        return Err(CoreError::InstanceExists);
    }
    let mut instance = original.clone();
    instance.name = name;
    instance.last_opened_at = None;
    let copied = copy_tree(&source, &destination, true)
        .and_then(|_| super::icon::duplicate_cached_icon(&source, &destination, &original));
    if let Err(error) = copied {
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        return Err(error);
    }
    if let Err(error) = write_instance(database, &destination, &instance).await {
        fs::remove_dir_all(&destination)?;
        return Err(error);
    }
    let entry = entry_from(&destination, new_folder, instance);
    if let Err(error) = database
        .duplicate_modpack(instances_dir, folder, &entry.folder)
        .await
    {
        let destination = instances_dir.join(&entry.folder);
        if let Err(cleanup) = tokio::fs::remove_dir_all(destination).await {
            return Err(std::io::Error::other(format!("Could not duplicate modpack index: {error}; could not remove incomplete copy: {cleanup}")).into());
        }
        database
            .delete_instance_profile(&instances_dir.join(&entry.folder))
            .await?;
        return Err(error);
    }
    Ok(entry)
}

/// Records that the instance was opened now (RFC 3339 UTC on the profile).
pub async fn touch_last_opened(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    instance.last_opened_at = Some(utc_now_rfc3339()?);
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
pub async fn delete_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<()> {
    let _guard = profile_guard().await;
    single_component(folder)?;
    let dir = instances_dir.join(folder);
    match tokio::fs::remove_dir_all(&dir).await {
        Ok(()) => {}
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && !tokio::fs::try_exists(&dir).await? => {}
        Err(error) => return Err(error.into()),
    }
    database.delete_instance_profile(&dir).await
}
/// Writes launch personalization while preserving the name and version.
pub async fn update_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    settings: &InstanceSettings,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
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
    if settings.override_memory {
        crate::settings::validate_memory_allocation(
            settings.max_memory_mb,
            crate::total_memory_mb(),
        )?;
    }
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
/// Points the instance at another game version. Does not download files.
pub async fn update_instance_version(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    version_id: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    single_component(version_id)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    instance.version_id = Some(version_id.to_string());
    validate(&instance)?;
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
