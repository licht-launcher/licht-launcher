use super::{
    GlobalLaunch, Instance, InstanceEntry, available_folder, display_name, entry_from,
    instance_dir, instance_from_template, load_instance, profile_guard, single_component,
    write_instance,
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
    /// A modpack archive the user picked from disk, staged in the installation workspace.
    ModpackFile { source: ModpackSource },
}

impl InstanceSource {
    pub(crate) fn game_version(&self) -> Option<&str> {
        match self {
            Self::Vanilla { game_version } | Self::Loader { game_version, .. } => {
                Some(game_version)
            }
            Self::Modpack { .. } | Self::ModpackFile { .. } => None,
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
    /// What the store said about the pack when it was installed. Older instances and
    /// installs whose store details could not be read have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub modpack: Option<LinkedModpack>,
    /// The base an instance had before a base change that has not finished, so
    /// cancelling the change can put it back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub previous_base: Option<PreviousBase>,
}

/// What a base change replaces: the launch version and the installation source, if
/// the instance had one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PreviousBase {
    pub version_id: Option<String>,
    pub source: Option<InstanceSource>,
}

/// A store pack's details as of its install, so the UI can show them without a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LinkedModpack {
    pub name: String,
    pub author: Option<String>,
    /// The installed version's name, such as `15.0.0-beta.3`.
    pub version_name: String,
    /// RFC 3339 timestamp of the installed version's release, when the store has one.
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum InstancePreparationState {
    Preparing,
    Installing,
    Ready,
}

/// Writes the instance before any metadata or artifact requests begin.
pub async fn create_pending_instance(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    name: &str,
    source: InstanceSource,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    source.validate()?;
    let name = display_name(name)?;
    fs::create_dir_all(instances_dir)?;
    let folder = available_folder(instances_dir, &name)?;
    let dir = instances_dir.join(&folder);
    fs::create_dir(&dir)?;
    let mut instance = instance_from_template(name, String::new(), template);
    instance.version_id = source.game_version().map(str::to_string);
    instance.installation = Some(InstanceInstallation {
        source,
        state: InstancePreparationState::Preparing,
        modpack: None,
        previous_base: None,
    });
    if let Err(error) = write_instance(database, &dir, &instance).await {
        fs::remove_dir(&dir)?;
        return Err(error);
    }
    Ok(entry_from(&dir, folder, instance))
}
pub async fn complete_instance_preparation(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    if instance.version_id.is_none() {
        return Err(CoreError::InstancePreparing);
    }
    if let Some(installation) = instance.installation.as_mut() {
        if installation.state == InstancePreparationState::Preparing {
            return Err(CoreError::InstancePreparing);
        }
        installation.state = InstancePreparationState::Ready;
        installation.previous_base = None;
    }
    crate::modpacks::workspace::cleanup(&dir)?;
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}
/// Keeps the store details of the pack being installed on the instance.
pub(crate) async fn record_linked_modpack(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    modpack: LinkedModpack,
) -> Result<()> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    let Some(installation) = instance.installation.as_mut() else {
        return Ok(());
    };
    installation.modpack = Some(modpack);
    write_instance(database, &dir, &instance).await
}

/// Turns a ready modpack instance into a regular one. Its files stay, but the
/// launcher forgets the pack: none of its content counts as pack content any more,
/// repairs stop restoring pack files, and its game version can change.
pub async fn unlink_instance_modpack(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    let Some(installation) = &instance.installation else {
        return Err(CoreError::InstanceNotLinked);
    };
    if !matches!(
        installation.source,
        InstanceSource::Modpack { .. } | InstanceSource::ModpackFile { .. }
    ) {
        return Err(CoreError::InstanceNotLinked);
    }
    if installation.state != InstancePreparationState::Ready {
        return Err(CoreError::InstancePreparing);
    }
    // Rows first: a failure here leaves the link in place, so the user can retry.
    database.delete_modpack(instances_dir, folder).await?;
    instance.installation = None;
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}

/// Starts moving a ready, unlinked instance to another platform or game version. The
/// instance waits in `Preparing` with `target` as its source until the change job
/// finishes; its current base is kept so cancelling can restore it.
pub async fn begin_instance_base_change(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    target: InstanceSource,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    if !matches!(
        target,
        InstanceSource::Vanilla { .. } | InstanceSource::Loader { .. }
    ) {
        return Err(CoreError::InstancePreparing);
    }
    target.validate()?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    let previous = match instance.installation.take() {
        None => None,
        Some(installation) => {
            if matches!(
                installation.source,
                InstanceSource::Modpack { .. } | InstanceSource::ModpackFile { .. }
            ) {
                return Err(CoreError::InstanceLinked);
            }
            if installation.state != InstancePreparationState::Ready {
                return Err(CoreError::InstancePreparing);
            }
            Some(installation.source)
        }
    };
    instance.installation = Some(InstanceInstallation {
        source: target,
        state: InstancePreparationState::Preparing,
        modpack: None,
        previous_base: Some(PreviousBase {
            version_id: instance.version_id.clone(),
            source: previous,
        }),
    });
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}

/// Puts back the base a cancelled base change replaced. An instance with no change
/// waiting is left as it is.
pub(crate) async fn restore_instance_base(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
) -> Result<()> {
    let _guard = profile_guard().await;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    let Some(previous) = instance
        .installation
        .as_mut()
        .and_then(|installation| installation.previous_base.take())
    else {
        return Ok(());
    };
    instance.version_id = previous.version_id;
    instance.installation = previous.source.map(|source| InstanceInstallation {
        source,
        state: InstancePreparationState::Ready,
        modpack: None,
        previous_base: None,
    });
    write_instance(database, &dir, &instance).await
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
pub async fn configure_instance_version(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    folder: &str,
    version: &str,
) -> Result<InstanceEntry> {
    let _guard = profile_guard().await;
    single_component(version)?;
    let dir = instance_dir(instances_dir, folder)?;
    let mut instance = load_instance(database, &dir).await?;
    instance.version_id = Some(version.to_string());
    if let Some(installation) = instance.installation.as_mut() {
        installation.state = InstancePreparationState::Installing;
    }
    write_instance(database, &dir, &instance).await?;
    Ok(entry_from(&dir, folder.to_string(), instance))
}

#[cfg(test)]
mod tests {
    use licht_testkit::temp::TempDir;

    use super::*;
    use crate::{DatabaseService, ModpackSource, SharedCache};

    fn template() -> GlobalLaunch {
        GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: vec![],
            fullscreen: false,
            width: None,
            height: None,
        }
    }

    fn linked() -> LinkedModpack {
        LinkedModpack {
            name: "Fabulously Optimized".into(),
            author: Some("robotkoer".into()),
            version_name: "15.0.0-beta.3".into(),
            published_at: Some("2026-11-01T10:00:00Z".into()),
        }
    }

    #[tokio::test]
    async fn keeps_the_store_details_of_the_pack_through_the_rest_of_the_install() {
        let temp = TempDir::new("linked-modpack");
        let root = temp.path().to_path_buf();
        let service = DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("service");
        let client = service.client();
        let source = InstanceSource::Modpack {
            source: ModpackSource::Modrinth,
            project_id: "fo".into(),
            version_id: "v15".into(),
            icon_url: None,
        };
        let entry = create_pending_instance(&client, &root, "Pack", source, &template())
            .await
            .expect("create");
        assert_eq!(
            entry.installation.as_ref().expect("installation").modpack,
            None
        );

        record_linked_modpack(&client, &root, &entry.folder, linked())
            .await
            .expect("record");
        configure_instance_version(&client, &root, &entry.folder, "1.21.1")
            .await
            .expect("version");
        let entry = complete_instance_preparation(&client, &root, &entry.folder)
            .await
            .expect("ready");

        assert_eq!(
            entry.installation.expect("installation").modpack,
            Some(linked())
        );
        service.shutdown().await.expect("shutdown");
    }

    async fn ready_pack(client: &crate::DatabaseClient, root: &Path) -> InstanceEntry {
        let source = InstanceSource::Modpack {
            source: ModpackSource::Modrinth,
            project_id: "fo".into(),
            version_id: "v15".into(),
            icon_url: None,
        };
        let entry = create_pending_instance(client, root, "Pack", source, &template())
            .await
            .expect("create");
        configure_instance_version(client, root, &entry.folder, "1.21.1")
            .await
            .expect("version");
        complete_instance_preparation(client, root, &entry.folder)
            .await
            .expect("ready")
    }

    async fn service(root: &Path) -> DatabaseService {
        DatabaseService::start(
            crate::Database::open(&SharedCache::at(root))
                .await
                .expect("database"),
        )
        .await
        .expect("service")
    }

    #[tokio::test]
    async fn unlinking_keeps_the_files_and_forgets_the_pack() {
        let temp = TempDir::new("unlink-modpack");
        let root = temp.path().to_path_buf();
        let service = service(&root).await;
        let client = service.client();
        let entry = ready_pack(&client, &root).await;
        let dir = root.join(&entry.folder);
        fs::create_dir_all(dir.join("mods")).expect("mods");
        fs::write(dir.join("mods").join("sodium.jar"), b"mod").expect("mod");
        client
            .replace_modpack_override_files(&root, &entry.folder, vec!["options.txt".into()])
            .await
            .expect("overrides");

        let unlinked = unlink_instance_modpack(&client, &root, &entry.folder)
            .await
            .expect("unlink");

        assert_eq!(unlinked.installation, None);
        assert_eq!(unlinked.version_id.as_deref(), Some("1.21.1"));
        assert!(dir.join("mods").join("sodium.jar").is_file());
        assert!(
            client
                .modpack_override_files(&root, &entry.folder)
                .await
                .expect("overrides")
                .is_empty()
        );
        let reloaded = load_instance(&client, &dir).await.expect("reload");
        assert_eq!(reloaded.installation, None);
        assert!(matches!(
            unlink_instance_modpack(&client, &root, &entry.folder).await,
            Err(CoreError::InstanceNotLinked)
        ));
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn refuses_to_unlink_a_pack_that_is_still_installing() {
        let temp = TempDir::new("unlink-preparing");
        let root = temp.path().to_path_buf();
        let service = service(&root).await;
        let client = service.client();
        let source = InstanceSource::ModpackFile {
            source: ModpackSource::Modrinth,
        };
        let entry = create_pending_instance(&client, &root, "Pack", source, &template())
            .await
            .expect("create");

        assert!(matches!(
            unlink_instance_modpack(&client, &root, &entry.folder).await,
            Err(CoreError::InstancePreparing)
        ));
        assert!(
            load_instance(&client, &root.join(&entry.folder))
                .await
                .expect("reload")
                .installation
                .is_some()
        );
        service.shutdown().await.expect("shutdown");
    }

    #[test]
    fn reads_an_installation_saved_before_store_details_were_kept() {
        let installation: InstanceInstallation = serde_json::from_str(
            r#"{"source":{"type":"modpackFile","source":"modrinth"},"state":"ready"}"#,
        )
        .expect("older installation");
        assert_eq!(installation.modpack, None);
        let saved = serde_json::to_value(&installation).expect("serialize");
        assert!(saved.get("modpack").is_none());
    }

    #[tokio::test]
    async fn a_finished_base_change_keeps_the_new_base_and_forgets_the_old_one() {
        let temp = TempDir::new("base-change");
        let root = temp.path().to_path_buf();
        let service = DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("service");
        let client = service.client();
        let vanilla = InstanceSource::Vanilla {
            game_version: "1.21.1".into(),
        };
        let entry = create_pending_instance(&client, &root, "Survival", vanilla, &template())
            .await
            .expect("create");
        configure_instance_version(&client, &root, &entry.folder, "1.21.1")
            .await
            .expect("version");
        complete_instance_preparation(&client, &root, &entry.folder)
            .await
            .expect("ready");
        let fabric = InstanceSource::Loader {
            platform: LoaderPlatform::Fabric,
            game_version: "1.21.4".into(),
            channel: LoaderChannel::Stable,
            loader_version: None,
        };

        let changing = begin_instance_base_change(&client, &root, &entry.folder, fabric.clone())
            .await
            .expect("begin");
        assert_eq!(
            changing
                .installation
                .as_ref()
                .and_then(|installation| installation.previous_base.clone()),
            Some(PreviousBase {
                version_id: Some("1.21.1".into()),
                source: Some(InstanceSource::Vanilla {
                    game_version: "1.21.1".into()
                }),
            })
        );
        configure_instance_version(
            &client,
            &root,
            &entry.folder,
            "fabric-loader-0.16.10-1.21.4",
        )
        .await
        .expect("version");
        let done = complete_instance_preparation(&client, &root, &entry.folder)
            .await
            .expect("ready");

        let installation = done.installation.expect("installation");
        assert_eq!(installation.source, fabric);
        assert_eq!(installation.previous_base, None);
        assert_eq!(
            done.version_id.as_deref(),
            Some("fabric-loader-0.16.10-1.21.4")
        );
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn a_linked_instance_cannot_change_its_base() {
        let temp = TempDir::new("base-change-linked");
        let root = temp.path().to_path_buf();
        let service = DatabaseService::start(
            crate::Database::open(&SharedCache::at(&root))
                .await
                .expect("database"),
        )
        .await
        .expect("service");
        let client = service.client();
        let source = InstanceSource::Modpack {
            source: ModpackSource::Modrinth,
            project_id: "fo".into(),
            version_id: "v15".into(),
            icon_url: None,
        };
        let entry = create_pending_instance(&client, &root, "Pack", source, &template())
            .await
            .expect("create");
        configure_instance_version(&client, &root, &entry.folder, "1.21.1")
            .await
            .expect("version");
        complete_instance_preparation(&client, &root, &entry.folder)
            .await
            .expect("ready");
        let vanilla = InstanceSource::Vanilla {
            game_version: "1.21.4".into(),
        };

        assert!(matches!(
            begin_instance_base_change(&client, &root, &entry.folder, vanilla).await,
            Err(CoreError::InstanceLinked)
        ));
        service.shutdown().await.expect("shutdown");
    }
}
