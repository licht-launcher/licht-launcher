use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use licht_core::{
    CoreError, DownloadJobOperation, DownloadJobStatus, DownloadManager, SharedCache, game_cache,
    list_instances,
};
use tokio::sync::mpsc;

use super::instances::{instance_refs, resolve_error, resolve_instance};
use crate::host::host_environment;
use crate::progress::drain_download_progress;

#[derive(Debug, Args)]
pub struct RepairArgs {
    /// Instance folder or unique display name to repair.
    #[arg(
        long = "instance",
        value_name = "NAME_OR_FOLDER",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub instance: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

pub async fn repair(args: RepairArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let environment = host_environment()?;
    let client = reqwest::Client::builder()
        .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let settings_db = pool.clone();
    let database = licht_core::DatabaseService::start(pool).await?;
    database.client().recover_download_jobs(&cache).await?;

    let root = cache.instances_dir()?;
    let entries = list_instances(&database.client(), &root).await?;
    let refs = instance_refs(&entries);
    let resolved = resolve_instance(&args.instance, &refs)
        .map_err(|error| resolve_error(&args.instance, error))?;
    let folder = resolved.folder.clone();

    let manager = Arc::new(DownloadManager::new(
        settings_db,
        cache,
        environment,
        client,
        database.client(),
    ));

    let mut events = manager.subscribe();
    let (progress_tx, mut progress_rx) = mpsc::channel(32);
    let progress_task = tokio::spawn(async move {
        while let Ok(snapshot) = events.recv().await {
            let _ = progress_tx.send(snapshot.progress).await;
        }
    });
    let progress_printer = tokio::spawn(async move {
        drain_download_progress(&mut progress_rx).await;
    });

    let result = async {
        let snapshot = manager
            .prepare(&folder, DownloadJobOperation::Repair)
            .await?;
        match snapshot.status {
            DownloadJobStatus::Completed => {
                println!("repaired\t{folder}");
                Ok(0)
            }
            DownloadJobStatus::Failed => Err(CoreError::Io(std::io::Error::other(
                snapshot
                    .error
                    .unwrap_or_else(|| format!("repair of `{folder}` failed")),
            ))),
            other => Err(CoreError::Io(std::io::Error::other(format!(
                "repair of `{folder}` ended with status {other:?}"
            )))),
        }
    }
    .await;

    drop(manager);
    let _ = progress_task.await;
    let _ = progress_printer.await;
    database.shutdown().await?;
    result
}
