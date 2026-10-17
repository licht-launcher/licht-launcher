use std::path::{Path, PathBuf};

use licht_core::{
    JavaDetections, JavaPaths, JavaStatus, Settings, SettingsSnapshot, browse_start_directory,
    detect_known_javas, runtime_java_paths, settings_snapshot,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use super::{LauncherPaths, failure, host_environment};

#[tauri::command]
pub fn get_settings(paths: State<'_, LauncherPaths>) -> Result<SettingsSnapshot, String> {
    settings_snapshot(&paths.settings).map_err(failure)
}

#[tauri::command]
pub fn save_settings(
    paths: State<'_, LauncherPaths>,
    settings: Settings,
) -> Result<Settings, String> {
    licht_core::save_settings(&paths.settings, &settings).map_err(failure)
}

#[tauri::command]
pub fn runtime_java(paths: State<'_, LauncherPaths>) -> Result<JavaPaths, String> {
    let environment = host_environment().map_err(failure)?;
    runtime_java_paths(&paths.game, &environment).map_err(failure)
}

#[tauri::command]
pub async fn java_installation_status(paths: JavaPaths) -> Result<JavaStatus, String> {
    tauri::async_runtime::spawn_blocking(move || licht_core::java_installation_status(&paths))
        .await
        .map_err(|_| "Java check failed".to_string())
}

#[tauri::command]
pub async fn detect_java_installations(
    paths: State<'_, LauncherPaths>,
) -> Result<JavaDetections, String> {
    let game = paths.game.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let environment = host_environment().map_err(failure)?;
        Ok(detect_known_javas(&game, environment.os))
    })
    .await
    .map_err(|_| "Java check failed".to_string())?
}

#[tauri::command]
pub async fn install_recommended_java(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    major: u32,
) -> Result<Option<String>, String> {
    downloads.inner().install_java(major).await.map_err(failure)
}

fn picked_path(file: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<String>, String> {
    let Some(file) = file else {
        return Ok(None);
    };
    match file.into_path() {
        Ok(path) => Ok(Some(path.display().to_string())),
        Err(error) => Err(error.to_string()),
    }
}

fn starting_directory(current: &str, fallback: &Path) -> Result<PathBuf, String> {
    let directory = browse_start_directory(current, fallback);
    if !directory.is_dir() {
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    }
    Ok(directory)
}

fn owned_file_dialog(app: &AppHandle) -> FileDialogBuilder<tauri::Wry> {
    let mut dialog = app.dialog().file();
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    dialog
}

#[tauri::command]
pub async fn browse_java(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let directory = starting_directory(&path, &paths.game.root().join("runtime"))?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&app)
        .set_directory(&directory)
        .pick_file(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver.await.map_err(|_| "browse failed".to_string())?;
    picked_path(file)
}

#[tauri::command]
pub async fn browse_application_directory(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let Some(fallback) = paths.settings.root().parent() else {
        return Err("application directory is missing".to_string());
    };
    let directory = starting_directory(&path, fallback)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&app)
        .set_directory(&directory)
        .pick_folder(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver.await.map_err(|_| "browse failed".to_string())?;
    picked_path(file)
}
