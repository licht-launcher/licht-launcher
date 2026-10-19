use std::sync::Arc;

use licht_core::{DownloadJobOperation, DownloadJobSnapshot, DownloadManager, InstallProgress};
use tauri::{AppHandle, Emitter, State};

use super::{LauncherPaths, failure};

#[tauri::command]
pub async fn list_download_jobs(
    paths: State<'_, LauncherPaths>,
) -> Result<Vec<DownloadJobSnapshot>, String> {
    let settings = paths.settings.clone();
    let game = paths.game.clone();
    tauri::async_runtime::spawn_blocking(move || licht_core::list_download_jobs(&settings, &game))
        .await
        .map_err(|_| "download job listing task failed".to_string())?
        .map_err(failure)
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

pub fn forward_events(app: AppHandle, downloads: Arc<DownloadManager>) {
    let mut events = downloads.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
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
