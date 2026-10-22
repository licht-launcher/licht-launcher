use futures_util::future::BoxFuture;

use crate::{
    CoreError, DownloadJobOperation, InstallationContext, InstancePreparationState, InstanceSource,
    Result, complete_instance_preparation, configure_instance_version, install_curseforge_files,
    install_game_version, install_mrpack_files, instance_entry, load_instance,
};

use super::jobs::model::DownloadCheckpoint;
use super::manager::DownloadRuntime;

pub(crate) fn execute(
    runtime: DownloadRuntime,
    job: crate::DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        if let Some(major) = job.java_major {
            let executable = crate::install_temurin(
                &runtime.client,
                &runtime.game,
                major,
                &runtime.environment,
                crate::ADOPTIUM_API,
                context,
            )
            .await?;
            crate::set_java_path(&runtime.settings, major, &executable)?;
            return Ok(());
        }
        let folder = job.instance_folder.ok_or(CoreError::InstancePreparing)?;
        let root = runtime.game.instances_dir()?;
        let dir = runtime.game.instance_dir(&folder)?;
        let profile = load_instance(&runtime.database, &dir).await?;
        let verify_modpack = job.operation == DownloadJobOperation::Repair
            || profile
                .installation
                .as_ref()
                .is_some_and(|installation| installation.state != InstancePreparationState::Ready);
        if let Some(installation) = &profile.installation
            && installation.state == InstancePreparationState::Preparing
        {
            let version = match &installation.source {
                InstanceSource::Vanilla { game_version } => game_version.clone(),
                InstanceSource::Loader {
                    platform,
                    game_version,
                    channel,
                    loader_version,
                } => {
                    crate::loaders::prepare_platform_profile(
                        &runtime.client,
                        &runtime.game,
                        *platform,
                        game_version,
                        *channel,
                        loader_version.as_deref(),
                        &context,
                    )
                    .await?
                }
                source @ InstanceSource::Modpack { .. } => {
                    let entry = instance_entry(&runtime.database, &dir, &folder).await?;
                    let imported = crate::modpacks::prepare_modpack_instance(
                        &runtime.database,
                        &runtime.client,
                        &runtime.game,
                        &root,
                        &entry,
                        source,
                        &context,
                    )
                    .await?;
                    imported.version_id.ok_or(CoreError::InstancePreparing)?
                }
            };
            context.checkpoint().await?;
            configure_instance_version(&runtime.database, &root, &folder, &version).await?;
        } else if let Some(installation) = &profile.installation {
            crate::modpacks::restore_instance_icon(
                &runtime.client,
                &dir,
                &installation.source,
                &context,
            )
            .await?;
        }
        let profile = load_instance(&runtime.database, &dir).await?;
        let version = profile
            .version_id
            .as_deref()
            .ok_or(CoreError::InstancePreparing)?;
        context.resolved_version(version).await?;
        if !runtime
            .database
            .has_download_checkpoint(&runtime.game, &job.id, version, DownloadCheckpoint::Game)
            .await?
            || !runtime.game.version_json(version)?.is_file()
        {
            install_game_version(
                &runtime.client,
                &runtime.game,
                &runtime.settings,
                version,
                &runtime.environment,
                context.clone(),
            )
            .await?;
            context.checkpoint().await?;
            runtime
                .database
                .record_download_checkpoint(
                    &runtime.game,
                    &job.id,
                    version,
                    DownloadCheckpoint::Game,
                )
                .await?;
        }
        if verify_modpack
            && !runtime
                .database
                .has_download_checkpoint(
                    &runtime.game,
                    &job.id,
                    version,
                    DownloadCheckpoint::Modpack,
                )
                .await?
        {
            install_mrpack_files(
                &runtime.database,
                &runtime.game,
                &runtime.settings,
                &dir,
                context.clone(),
            )
            .await?;
            install_curseforge_files(&runtime.game, &runtime.settings, &dir, context.clone())
                .await?;
            context.checkpoint().await?;
            runtime
                .database
                .record_download_checkpoint(
                    &runtime.game,
                    &job.id,
                    version,
                    DownloadCheckpoint::Modpack,
                )
                .await?;
        }
        context.checkpoint().await?;
        complete_instance_preparation(&runtime.database, &root, &folder).await?;
        Ok(())
    })
}
