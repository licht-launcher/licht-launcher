use tauri::{AppHandle, State, WebviewWindow};
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

#[tauri::command]
pub fn quit_app(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Quit is only available in the launcher".to_string());
    }
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_build_diagnostics(
    window: WebviewWindow,
) -> Result<licht_core::BuildDiagnostics, String> {
    if window.label() != "main" {
        return Err("Build information is only available in the launcher".to_string());
    }
    tauri::async_runtime::spawn_blocking(|| {
        let webview = match tauri::webview_version() {
            Ok(version) => Some(version),
            Err(_) => {
                tracing::debug!("WebView runtime version is unavailable");
                None
            }
        };
        licht_core::build_diagnostics(
            env!("CARGO_PKG_VERSION"),
            option_env!("LICHT_BUILD_REVISION"),
            tauri::VERSION,
            webview,
            cfg!(debug_assertions),
        )
    })
    .await
    .map_err(|_| "Could not collect build and runtime information".to_string())
}
