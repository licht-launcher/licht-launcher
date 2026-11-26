use std::path::PathBuf;

use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

const REPOSITORY_URL: &str = "https://github.com/licht-launcher/licht-launcher";

#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|error| super::command_error(&error))
}

#[tauri::command]
pub fn open_feedback(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(format!("{REPOSITORY_URL}/issues/new/choose"), None::<&str>)
        .map_err(|error| super::command_error(&error))
}

#[tauri::command]
pub fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    let url = licht_core::external_https_url(&url).map_err(super::failure)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| super::command_error(&error))
}

#[tauri::command]
pub fn get_logs_folder() -> Result<String, String> {
    crate::logging::log_directory()
        .map(|directory| directory.to_string_lossy().into_owned())
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
pub fn open_logs_folder(app: AppHandle) -> Result<(), String> {
    let directory = crate::logging::log_directory().map_err(|error| format!("{error:#}"))?;
    let directory = existing_log_folder(directory)?;
    app.opener()
        .open_path(directory.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| super::command_error(&error))
}

/// Logging creates the folder at startup; it is missing only when that failed.
fn existing_log_folder(directory: PathBuf) -> Result<PathBuf, String> {
    if directory.is_dir() {
        Ok(directory)
    } else {
        Err("The log folder does not exist yet. Restart the launcher to create it.".to_string())
    }
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
    if !crate::settings_window::is_launcher_window(window.label()) {
        return Err("Build information is only available in the launcher".to_string());
    }
    tauri::async_runtime::spawn_blocking(collect_build_diagnostics)
        .await
        .map_err(|error| {
            super::command_failure("Could not collect build and runtime information", &error)
        })
}

/// Blocks while it queries the operating system and the WebView runtime.
pub(crate) fn collect_build_diagnostics() -> licht_core::BuildDiagnostics {
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
}

#[cfg(test)]
mod tests {
    use std::fs;

    use licht_testkit::temp::TempDir;

    use super::{existing_log_folder, get_logs_folder};

    #[test]
    fn the_log_folder_opens_only_when_it_exists() {
        let root = TempDir::new("log-folder-test");
        let logs = root.path().join("logs");
        fs::create_dir_all(&logs).expect("the log folder should be created");

        assert_eq!(existing_log_folder(logs.clone()), Ok(logs.clone()));
        fs::remove_dir_all(root.path()).expect("the test folder should be removed");
        assert_eq!(
            existing_log_folder(logs),
            Err(
                "The log folder does not exist yet. Restart the launcher to create it.".to_string()
            )
        );
    }

    #[test]
    fn the_log_folder_is_the_logs_directory_of_the_profile() {
        let profile =
            licht_core::SharedCache::system().expect("the profile directory should resolve");

        assert_eq!(
            get_logs_folder(),
            Ok(profile.root().join("logs").to_string_lossy().into_owned())
        );
    }
}
