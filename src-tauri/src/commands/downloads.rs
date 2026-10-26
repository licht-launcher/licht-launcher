use std::sync::Arc;

use licht_core::{DownloadJobOperation, DownloadJobSnapshot, DownloadManager, InstallProgress};
use tauri::{AppHandle, Emitter, State};

use super::{LauncherPaths, failure};

#[tauri::command]
pub async fn list_download_jobs(
    paths: State<'_, LauncherPaths>,
    database: State<'_, licht_core::DatabaseClient>,
) -> Result<Vec<DownloadJobSnapshot>, String> {
    database
        .list_download_jobs(&paths.game)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn reorder_download_jobs(
    downloads: State<'_, Arc<DownloadManager>>,
    job_ids: Vec<String>,
) -> Result<Vec<DownloadJobSnapshot>, String> {
    downloads.reorder(job_ids).await.map_err(failure)
}

#[tauri::command]
pub async fn storage_summary(
    paths: State<'_, LauncherPaths>,
) -> Result<licht_core::StorageSummary, String> {
    let game = paths.game.clone();
    tauri::async_runtime::spawn_blocking(move || {
        licht_core::storage_summary(&game).map_err(failure)
    })
    .await
    .map_err(|_| "Storage check failed".to_string())?
}

#[tauri::command]
pub async fn prepare_instance_download(
    downloads: State<'_, Arc<DownloadManager>>,
    folder: String,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot, String> {
    downloads
        .inner()
        .prepare(&folder, operation)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn pause_download_job(
    downloads: State<'_, Arc<DownloadManager>>,
    job_id: String,
) -> Result<DownloadJobSnapshot, String> {
    downloads.pause(&job_id).await.map_err(failure)
}

#[tauri::command]
pub async fn resume_download_job(
    downloads: State<'_, Arc<DownloadManager>>,
    job_id: String,
) -> Result<DownloadJobSnapshot, String> {
    downloads.inner().resume(&job_id).await.map_err(failure)
}

#[tauri::command]
pub async fn cancel_download_job(
    downloads: State<'_, Arc<DownloadManager>>,
    job_id: String,
) -> Result<DownloadJobSnapshot, String> {
    downloads.cancel(&job_id).await.map_err(failure)
}

#[tauri::command]
pub fn download_telemetry(
    downloads: State<'_, Arc<DownloadManager>>,
) -> licht_core::DownloadTelemetry {
    downloads.telemetry()
}

#[tauri::command]
pub async fn pause_all_downloads(downloads: State<'_, Arc<DownloadManager>>) -> Result<(), String> {
    downloads.pause_all().await.map_err(failure)
}

#[tauri::command]
pub async fn resume_all_downloads(
    downloads: State<'_, Arc<DownloadManager>>,
) -> Result<(), String> {
    downloads.inner().resume_all().await.map_err(failure)
}

pub fn forward_events(app: AppHandle, downloads: Arc<DownloadManager>) {
    let mut events = downloads.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            let event = tokio::select! {
                event = events.recv() => event,
                _ = ticks.tick() => {
                    if app.emit("download-telemetry", downloads.telemetry()).is_err() {
                        tracing::warn!("Failed to emit download telemetry");
                    }
                    continue;
                }
            };
            match event {
                Ok(snapshot) => {
                    if app.emit("download-job-updated", &snapshot).is_err() {
                        tracing::warn!("Failed to emit download job state");
                    }
                    if let Some(version) = &snapshot.version_id {
                        let _legacy_observers = app.emit(
                            "install-progress",
                            InstallProgress::tagged(version, snapshot.progress),
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if app.emit("download-jobs-invalidated", ()).is_err() {
                        tracing::warn!("Failed to invalidate download job state");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
