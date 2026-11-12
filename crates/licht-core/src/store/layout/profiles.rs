use std::collections::HashSet;
use std::fs;

use super::installation;
use super::progress::StartupReporter;
use crate::store::db::Database;
use crate::store::repositories::instances::InstanceRepository;
use crate::store::repositories::metadata::MetadataRepository;
use crate::{CoreError, Result, SharedCache};

const PROFILE_FILE: &str = "instance.json";

/// Registers the instances that 0.2.3 described only with an `instance.json` beside their
/// files. The migration copies those folders, but the launcher lists only SQLite profiles.
/// Runs once per instances directory; folders that already have a profile keep it.
pub(super) async fn import_legacy(
    database: &Database,
    game: &SharedCache,
    reporter: &StartupReporter,
) -> Result<()> {
    let instances = game.instances_dir()?;
    let key = format!("legacy-instance-profiles-v1:{}", instances.display());
    let mut connection = database.acquire().await?;
    if MetadataRepository::get(&mut connection, &key)
        .await?
        .as_deref()
        == Some("1")
    {
        return Ok(());
    }
    let directory = instances.to_str().ok_or(CoreError::CachePath)?;
    let registered = InstanceRepository::list(&mut connection, directory)
        .await?
        .into_iter()
        .map(|(folder, _)| folder)
        .collect::<HashSet<_>>();
    if instances.is_dir() {
        for item in fs::read_dir(&instances)? {
            reporter.checkpoint()?;
            let item = item?;
            let Some(folder) = item.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let profile = item.path().join(PROFILE_FILE);
            if !item.file_type()?.is_dir()
                || registered.contains(&folder)
                || !fs::symlink_metadata(&profile).is_ok_and(|metadata| metadata.is_file())
            {
                continue;
            }
            let instance = crate::instance::single_component(&folder)
                .and_then(|_| crate::parse_instance(&fs::read_to_string(&profile)?));
            let instance = match instance {
                Ok(instance) => instance,
                Err(error) => {
                    tracing::warn!(error = %error, "Keeping a legacy instance profile that could not be imported");
                    continue;
                }
            };
            InstanceRepository::write(&mut connection, directory, &folder, &instance).await?;
            // The database now holds the profile, so a leftover file changes nothing.
            if let Err(error) = fs::remove_file(&profile) {
                tracing::warn!(error = %error, "Could not remove an imported legacy instance profile");
            }
        }
    }
    installation::migrate_icons(&mut connection, &instances, reporter).await?;
    MetadataRepository::set(&mut connection, &key, "1").await?;
    Ok(())
}
