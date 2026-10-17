use licht_core::DownloadJobSnapshot;
use tauri::State;

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
