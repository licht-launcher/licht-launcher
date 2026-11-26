use std::path::{Path, PathBuf};

use licht_core::{
    JavaDetections, JavaPaths, JavaStatus, Settings, SettingsSnapshot, browse_start_directory,
    detect_known_javas, runtime_java_paths, settings_snapshot,
};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use super::{LauncherPaths, failure, host_environment};

#[tauri::command]
pub async fn get_settings(paths: State<'_, LauncherPaths>) -> Result<SettingsSnapshot, String> {
    settings_snapshot(&paths.settings).await.map_err(failure)
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    updates: State<'_, std::sync::Arc<crate::app_updates::Runtime>>,
    paths: State<'_, LauncherPaths>,
    settings: Settings,
) -> Result<Settings, String> {
    let saved = licht_core::save_settings(&paths.settings, &settings)
        .await
        .map_err(failure)?;
    // Saved, so every window must refresh even if a later step fails.
    super::notify_windows(&app, super::SETTINGS_CHANGED_EVENT);
    updates
        .preferences_changed(&app, saved.automatic_updates)
        .map_err(|error| {
            format!("Settings were saved, but background updates could not be notified: {error}")
        })?;
    crate::desktop::apply_preferences(&app, &saved).map_err(|error| {
        format!("Settings were saved, but launcher behavior could not be applied: {error}")
    })?;
    Ok(saved)
}

/// The value this process started with; a saved change applies after a restart.
#[tauri::command]
pub fn applied_hardware_acceleration(runtime: State<'_, crate::desktop::Runtime>) -> bool {
    runtime.hardware_acceleration
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
        .map_err(|error| super::command_failure("Java check failed", &error))
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
    .map_err(|error| super::command_failure("Java check failed", &error))?
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
        Err(error) => Err(super::command_error(&error)),
    }
}

fn starting_directory(current: &str, fallback: &Path) -> Result<PathBuf, String> {
    let directory = browse_start_directory(current, fallback);
    if !directory.is_dir() {
        std::fs::create_dir_all(&directory).map_err(|error| super::command_error(&error))?;
    }
    Ok(directory)
}

fn owned_file_dialog(window: &WebviewWindow) -> FileDialogBuilder<tauri::Wry> {
    window.dialog().file().set_parent(window)
}

#[tauri::command]
pub async fn browse_java(
    window: WebviewWindow,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let directory = starting_directory(&path, &paths.game.root().join("runtime"))?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&window)
        .set_directory(&directory)
        .pick_file(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver
        .await
        .map_err(|error| super::command_failure("browse failed", &error))?;
    picked_path(file)
}

#[tauri::command]
pub async fn browse_application_directory(
    window: WebviewWindow,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let Some(fallback) = paths.settings.cache().root().parent() else {
        return Err("application directory is missing".to_string());
    };
    let directory = starting_directory(&path, fallback)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&window)
        .set_directory(&directory)
        .pick_folder(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver
        .await
        .map_err(|error| super::command_failure("browse failed", &error))?;
    picked_path(file)
}
