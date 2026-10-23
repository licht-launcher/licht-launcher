use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

const REPOSITORY_URL: &str = "https://github.com/licht-launcher/licht-launcher";

/// Title bar mode chosen when the main window was created (restart to change).
#[derive(Clone, Copy)]
pub struct TitleBarMode {
    pub custom: bool,
}

#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_feedback(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(format!("{REPOSITORY_URL}/issues/new/choose"), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn custom_title_bar_active(mode: State<'_, TitleBarMode>) -> bool {
    mode.custom
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.request_restart();
}
