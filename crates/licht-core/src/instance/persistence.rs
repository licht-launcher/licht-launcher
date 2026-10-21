use super::{Instance, InstancePreparationState, SCHEMA, display_name, single_component};
use crate::{CoreError, Result};
use serde::Deserialize;
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
/// Reads the SQLite profile for an existing instance directory.
pub async fn load_instance(database: &crate::DatabaseClient, dir: &Path) -> Result<Instance> {
    if !dir.is_dir() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "instance directory is missing",
        )));
    }
    database.instance_profile(dir).await
}
pub(super) async fn write_instance(
    database: &crate::DatabaseClient,
    dir: &Path,
    instance: &Instance,
) -> Result<()> {
    database.write_instance_profile(dir, instance).await
}
pub(crate) fn validate(instance: &Instance) -> Result<()> {
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
