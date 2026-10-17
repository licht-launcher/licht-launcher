use serde::{Deserialize, Serialize};
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
