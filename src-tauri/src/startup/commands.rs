use std::sync::Arc;

use licht_core::{StartupPreviewScenario, StartupProgress};
use tauri::{AppHandle, State, WebviewWindow};

use super::state::StartupRuntime;

#[tauri::command]
pub fn get_startup_progress(
    window: WebviewWindow,
    runtime: State<'_, Arc<StartupRuntime>>,
) -> Result<StartupProgress, String> {
    let session = runtime
        .session(window.label())
        .map_err(|error| error.to_string())?;
    session.acknowledge();
    session.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn open_startup_preview(
    app: AppHandle,
    window: WebviewWindow,
    runtime: State<'_, Arc<StartupRuntime>>,
    scenario: StartupPreviewScenario,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Startup tests must be opened from the launcher".to_string());
    }
    let runtime = Arc::clone(runtime.inner());
    // WebView2 creation must not block the synchronous IPC/UI thread on Windows.
    tauri::async_runtime::spawn_blocking(move || super::preview::open(app, runtime, scenario))
        .await
        .map_err(|_| "Could not join startup preview creation".to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn restart_startup_preview(
    app: AppHandle,
    window: WebviewWindow,
    downloads: State<'_, Arc<licht_core::DownloadManager>>,
    scenario: StartupPreviewScenario,
) -> Result<(), String> {
    if window.label() != "main" || !cfg!(debug_assertions) {
        return Err("Startup restart tests require the development launcher".to_string());
    }
    downloads
        .suspend_all()
        .await
        .map_err(|error| error.to_string())?;
    licht_core::request_startup_preview(scenario).map_err(|error| error.to_string())?;
    app.request_restart();
    Ok(())
}

#[tauri::command]
pub async fn request_update_restart(
    app: AppHandle,
    window: WebviewWindow,
    downloads: State<'_, Arc<licht_core::DownloadManager>>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Updates must be requested from the launcher".to_string());
    }
    downloads
        .suspend_all()
        .await
        .map_err(|error| error.to_string())?;
    licht_core::request_startup_update(env!("CARGO_PKG_VERSION"))
        .map_err(|error| error.to_string())?;
    app.request_restart();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_command_requires_async_ipc_dispatch() {
        // WebView2 creation deadlocks on Windows when Tauri dispatches a synchronous command.
        fn require_async<F, Output>(_: F)
        where
            F: Fn(
                AppHandle,
                WebviewWindow,
                State<'static, Arc<StartupRuntime>>,
                StartupPreviewScenario,
            ) -> Output,
            Output: std::future::Future<Output = Result<(), String>>,
        {
        }
        require_async(open_startup_preview);
    }
}
