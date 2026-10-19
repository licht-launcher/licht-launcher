use super::{
    FILE_NAME, Instance, InstancePreparationState, SCHEMA, default_instance, directory_name,
    display_name, single_component,
};
use crate::{CoreError, Result};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
struct SchemaHeader {
    schema: u32,
}
pub fn parse_instance(json: &str) -> Result<Instance> {
    let header: SchemaHeader = serde_json::from_str(json).map_err(CoreError::Instance)?;
    if header.schema != 1 && header.schema != SCHEMA {
        return Err(CoreError::InstanceSchema {
            schema: header.schema,
        });
    }
    let instance: Instance = serde_json::from_str(json).map_err(CoreError::Instance)?;
    validate(&instance)?;
    Ok(instance)
}
/// Reads `instance.json` in `dir`. A missing file is initialized with the current schema.
/// The directory itself is not created.
pub fn load_instance(dir: &Path) -> Result<Instance> {
    if !dir.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    let path = dir.join(FILE_NAME);
    if path.is_file() {
        let json = fs::read_to_string(&path)?;
        return parse_instance(&json);
    }
    let name = directory_name(dir)?;
    let instance = default_instance(name.clone(), name);
    write_instance(dir, &instance)?;
    Ok(instance)
}
pub(super) fn write_instance(dir: &Path, instance: &Instance) -> Result<()> {
    let body = serde_json::to_string_pretty(instance).map_err(CoreError::Instance)?;
    let temporary = dir.join(".instance.json.tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(body.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, dir.join(FILE_NAME))?;
    Ok(())
}
pub(super) fn validate(instance: &Instance) -> Result<()> {
    display_name(&instance.name)?;
    if let Some(version) = &instance.version_id {
        single_component(version)?;
    } else if !instance
        .installation
        .as_ref()
        .is_some_and(|installation| installation.state == InstancePreparationState::Preparing)
    {
        return Err(CoreError::InstancePreparing);
    }
    if let Some(installation) = &instance.installation {
        installation.source.validate()?;
    }
    if instance.min_memory_mb == 0
        || instance.max_memory_mb == 0
        || instance.min_memory_mb > instance.max_memory_mb
    {
        return Err(CoreError::InstanceMemory);
    }
    Ok(())
}
