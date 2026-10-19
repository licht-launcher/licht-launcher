use super::{
    GlobalLaunch, Instance, InstanceEntry, display_name, entry_from, folder_id, instance_dir,
    instance_from_template, load_instance, profile_guard, single_component, write_instance,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use ts_rs::TS;

use crate::{CoreError, LoaderChannel, LoaderPlatform, ModpackSource, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InstanceSource {
    Vanilla {
        #[serde(rename = "gameVersion")]
        game_version: String,
    },
    Loader {
        platform: LoaderPlatform,
        #[serde(rename = "gameVersion")]
        game_version: String,
        channel: LoaderChannel,
        #[serde(rename = "loaderVersion")]
        loader_version: Option<String>,
    },
    Modpack {
        source: ModpackSource,
        #[serde(rename = "projectId")]
        project_id: String,
        #[serde(rename = "versionId")]
        version_id: String,
        #[serde(rename = "iconUrl")]
        icon_url: Option<String>,
    },
}

impl InstanceSource {
    pub(crate) fn game_version(&self) -> Option<&str> {
        match self {
            Self::Vanilla { game_version } | Self::Loader { game_version, .. } => {
                Some(game_version)
            }
            Self::Modpack { .. } => None,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(version) = self.game_version() {
            super::single_component(version)?;
        }
        match self {
            Self::Loader {
                platform: LoaderPlatform::Vanilla,
                ..
            } => Err(CoreError::LoaderVersionMissing),
            Self::Loader {
                channel: LoaderChannel::Manual,
                loader_version: None,
                ..
            } => Err(CoreError::LoaderVersionMissing),
            Self::Loader {
                loader_version: Some(version),
                ..
            } if version.is_empty()
                || !version
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte)) =>
            {
                Err(CoreError::LoaderVersionMissing)
            }
            Self::Modpack {
                project_id,
                version_id,
                icon_url,
                ..
            } => {
                for value in [project_id, version_id] {
                    if value.is_empty()
                        || !value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                    {
                        return Err(CoreError::ModpackMeta {
                            detail: "invalid project or version identifier".into(),
                        });
                    }
                }
                if icon_url
                    .as_deref()
                    .is_some_and(|url| !crate::icon_url_allowed(url))
                {
                    return Err(CoreError::ModpackMeta {
                        detail: "invalid icon URL".into(),
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInstallation {
    pub source: InstanceSource,
    pub state: InstancePreparationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum InstancePreparationState {
    Preparing,
    Installing,
    Ready,
}

/// Writes the instance before any metadata or artifact requests begin.
pub fn create_pending_instance(
    instances_dir: &Path,
    name: &str,
    source: InstanceSource,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    source.validate()?;
    let name = display_name(name)?;
    let folder = folder_id(&name)?;
    fs::create_dir_all(instances_dir)?;
    let dir = instances_dir.join(&folder);
    fs::create_dir(&dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            CoreError::InstanceExists
        } else {
            CoreError::Io(error)
        }
    })?;
    let mut instance = instance_from_template(name, String::new(), template);
    instance.version_id = source.game_version().map(str::to_string);
    instance.installation = Some(InstanceInstallation {
        source,
        state: InstancePreparationState::Preparing,
    });
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder, instance))
}
pub fn complete_instance_preparation(instances_dir: &Path, folder: &str) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    if instance.version_id.is_none() {
        return Err(CoreError::InstancePreparing);
    }
    if let Some(installation) = instance.installation.as_mut() {
        if installation.state == InstancePreparationState::Preparing {
            return Err(CoreError::InstancePreparing);
        }
        installation.state = InstancePreparationState::Ready;
    }
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
pub fn instance_version_for_launch(instance: &Instance) -> Result<&str> {
    if instance
        .installation
        .as_ref()
        .is_some_and(|installation| installation.state != InstancePreparationState::Ready)
    {
        return Err(CoreError::InstancePreparing);
    }
    instance
        .version_id
        .as_deref()
        .ok_or(CoreError::InstancePreparing)
}
pub fn configure_instance_version(
    instances_dir: &Path,
    folder: &str,
    version: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard()?;
    single_component(version)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(&dir)?;
    instance.version_id = Some(version.to_string());
    if let Some(installation) = instance.installation.as_mut() {
        installation.state = InstancePreparationState::Installing;
    }
    write_instance(&dir, &instance)?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
