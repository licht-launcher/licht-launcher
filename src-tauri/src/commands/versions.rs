use licht_core::{
    CoreError, DownloadProgress, SharedCache, VersionSummary, install_game_version,
    is_version_ready, list_version_summaries, load_instance,
};
use tauri::{AppHandle, State};
use tokio::sync::mpsc;

use crate::progress::ProgressThrottle;

use super::install_progress::emit_install_progress;
use super::modpacks::install_modpack_files;
use super::{LauncherPaths, failure, host_environment, http_client};

#[tauri::command]
pub async fn list_versions(paths: State<'_, LauncherPaths>) -> Result<Vec<VersionSummary>, String> {
    let cache = paths.game.clone();
    let client = http_client().map_err(failure)?;
    list_version_summaries(&client, &cache)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn is_version_ready_cmd(
    paths: State<'_, LauncherPaths>,
    version_id: String,
) -> Result<bool, String> {
    Ok(is_version_ready(&paths.game, &version_id))
}

#[tauri::command]
pub async fn install_version(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    version_id: String,
) -> Result<String, String> {
    install_one(&app, &paths.game, &paths.settings, &version_id, &version_id)
        .await
        .map_err(failure)
}

/// Re-downloads game assets and modpack files for an instance (Repair).
#[tauri::command]
pub async fn repair_instance(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    let dir = paths.game.instance_dir(&folder).map_err(failure)?;
    let instance = load_instance(&dir).map_err(failure)?;
    install_one(
        &app,
        &paths.game,
        &paths.settings,
        &instance.version_id,
        &instance.version_id,
    )
    .await
    .map_err(failure)?;
    install_modpack_files(app, paths, folder).await
}

async fn install_one(
    app: &AppHandle,
    game: &SharedCache,
    settings: &SharedCache,
    version_id: &str,
    progress_version_id: &str,
) -> Result<String, CoreError> {
    emit_install_progress(
        app,
        progress_version_id,
        DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        },
    );

    let cache = game.clone();
    let settings = settings.clone();
    let client = http_client()?;
    let (progress, mut incoming) = mpsc::channel(4096);
    let id = version_id.to_string();
    let environment = host_environment()?;
    let installing = tauri::async_runtime::spawn(async move {
        install_game_version(&client, &cache, &settings, &id, &environment, progress).await
    });

    let mut throttle = ProgressThrottle::new();
    while let Some(progress) = incoming.recv().await {
        if throttle.should_emit(progress, std::time::Instant::now()) {
            emit_install_progress(app, progress_version_id, progress);
        }
    }

    let java = installing
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
    Ok(java.display().to_string())
}
