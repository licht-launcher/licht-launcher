use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::Context;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::{LauncherPaths, command_error, command_failure, failure};

const LABEL_PREFIX: &str = "instance-settings-";

/// Each instance has at most one settings window; labels are never reused.
#[derive(Debug, Default)]
struct Windows {
    next: u64,
    by_folder: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Next {
    Create(String),
    Show(String),
}

impl Windows {
    fn request(&mut self, folder: &str, window_exists: impl Fn(&str) -> bool) -> Next {
        if let Some(label) = self.by_folder.get(folder)
            && window_exists(label)
        {
            return Next::Show(label.clone());
        }
        let label = format!("{LABEL_PREFIX}{}", self.next);
        self.next += 1;
        self.by_folder.insert(folder.to_string(), label.clone());
        Next::Create(label)
    }

    fn folder_of(&self, label: &str) -> Option<String> {
        self.by_folder
            .iter()
            .find(|(_, open)| open.as_str() == label)
            .map(|(folder, _)| folder.clone())
    }

    fn label_of(&self, folder: &str) -> Option<String> {
        self.by_folder.get(folder).cloned()
    }

    fn labels(&self) -> Vec<String> {
        self.by_folder.values().cloned().collect()
    }

    fn closed(&mut self, label: &str) {
        self.by_folder.retain(|_, open| open != label);
    }
}

/// Held while creating a window, so two quick requests cannot both create it.
static WINDOWS: Mutex<Windows> = Mutex::new(Windows {
    next: 0,
    by_folder: BTreeMap::new(),
});

fn windows() -> anyhow::Result<std::sync::MutexGuard<'static, Windows>> {
    WINDOWS
        .lock()
        .map_err(|_| anyhow::anyhow!("instance settings window lock failed"))
}

pub fn is_instance_settings_window(label: &str) -> bool {
    label.starts_with(LABEL_PREFIX)
}

fn open(app: &AppHandle, folder: &str) -> anyhow::Result<()> {
    let mut windows = windows()?;
    match windows.request(folder, |label| app.get_webview_window(label).is_some()) {
        Next::Show(label) => {
            let window = app
                .get_webview_window(&label)
                .context("The instance settings window closed while opening; try again")?;
            window.unminimize()?;
            window.show()?;
            window.set_focus()?;
        }
        Next::Create(label) => {
            let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::default())
                .title("Instance Settings - Licht Launcher")
                // The same size as the launcher settings window.
                .inner_size(896.0, 664.0)
                .min_inner_size(760.0, 520.0)
                .center();
            if let Err(error) = crate::build_launcher_style_window(app, builder) {
                windows.closed(&label);
                return Err(error.into());
            }
        }
    }
    Ok(())
}

pub fn forget(label: &str) {
    match windows() {
        Ok(mut windows) => windows.closed(label),
        Err(error) => tracing::error!(%error, "Could not reset an instance settings window"),
    }
}

/// Each page saves its last edit, then closes itself without being able to refuse.
pub fn close_all(app: &AppHandle) {
    let labels = match windows() {
        Ok(windows) => windows.labels(),
        Err(error) => {
            tracing::error!(%error, "Could not list the instance settings windows");
            return;
        }
    };
    for label in labels {
        crate::settings_window::close_after_last_edit(app, &label);
    }
}

/// A deleted instance has nothing left to save.
pub fn close_for(app: &AppHandle, folder: &str) {
    let label = match windows() {
        Ok(windows) => windows.label_of(folder),
        Err(error) => {
            tracing::error!(%error, "Could not find the instance settings window");
            return;
        }
    };
    if let Some(window) = label.and_then(|label| app.get_webview_window(&label)) {
        crate::settings_window::destroy(&window);
    }
}

#[tauri::command]
pub async fn open_instance_settings_window(
    app: AppHandle,
    window: WebviewWindow,
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Instance settings can only be opened from the launcher".to_string());
    }
    if !paths.game.instance_dir(&folder).map_err(failure)?.is_dir() {
        return Err("instance directory is missing".to_string());
    }
    // WebView2 creation must not block the synchronous IPC/UI thread on Windows.
    tauri::async_runtime::spawn_blocking(move || open(&app, &folder))
        .await
        .map_err(|error| {
            command_failure("Could not join instance settings window creation", &error)
        })?
        .map_err(|error| command_error(error.as_ref()))
}

// Async: on the UI thread it could deadlock with `open` creating the window.
#[tauri::command]
pub async fn instance_settings_window_ready(window: WebviewWindow) -> Result<String, String> {
    let windows = windows().map_err(|error| command_error(error.as_ref()))?;
    windows
        .folder_of(window.label())
        .ok_or_else(|| "Only an instance settings window can report that it is ready".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_instance_settings_window_labels() {
        assert!(is_instance_settings_window("instance-settings-0"));
        assert!(!is_instance_settings_window("settings"));
        assert!(!is_instance_settings_window("main"));
    }

    #[test]
    fn gives_each_instance_its_own_window() {
        let mut windows = Windows::default();
        let first = windows.request("Survival", |_| false);
        let second = windows.request("Creative", |_| false);
        assert_eq!(first, Next::Create("instance-settings-0".to_string()));
        assert_eq!(second, Next::Create("instance-settings-1".to_string()));
        assert_eq!(
            windows.folder_of("instance-settings-1").as_deref(),
            Some("Creative")
        );
    }

    #[test]
    fn shows_the_open_window_of_an_instance_again() {
        let mut windows = Windows::default();
        windows.request("Survival", |_| false);
        assert_eq!(
            windows.request("Survival", |_| true),
            Next::Show("instance-settings-0".to_string())
        );
    }

    #[test]
    fn creates_a_new_window_after_the_old_one_closed() {
        let mut windows = Windows::default();
        windows.request("Survival", |_| false);
        windows.closed("instance-settings-0");
        assert_eq!(windows.folder_of("instance-settings-0"), None);
        assert_eq!(
            windows.request("Survival", |_| false),
            Next::Create("instance-settings-1".to_string())
        );
    }

    #[test]
    fn replaces_a_window_closed_without_notice() {
        let mut windows = Windows::default();
        windows.request("Survival", |_| false);
        assert_eq!(
            windows.request("Survival", |_| false),
            Next::Create("instance-settings-1".to_string())
        );
        assert_eq!(windows.folder_of("instance-settings-0"), None);
    }

    #[test]
    fn finds_the_windows_to_close() {
        let mut windows = Windows::default();
        windows.request("Survival", |_| false);
        windows.request("Creative", |_| false);
        assert_eq!(
            windows.label_of("Creative").as_deref(),
            Some("instance-settings-1")
        );
        assert_eq!(windows.label_of("Hardcore"), None);
        let mut labels = windows.labels();
        labels.sort();
        assert_eq!(labels, ["instance-settings-0", "instance-settings-1"]);
        windows.closed("instance-settings-0");
        assert_eq!(windows.labels(), ["instance-settings-1"]);
    }

    #[test]
    fn open_command_requires_async_ipc_dispatch() {
        // WebView2 creation deadlocks on Windows when Tauri dispatches a synchronous command.
        fn require_async<F, Output>(_: F)
        where
            F: Fn(AppHandle, WebviewWindow, State<'static, LauncherPaths>, String) -> Output,
            Output: std::future::Future<Output = Result<(), String>>,
        {
        }
        require_async(open_instance_settings_window);
    }
}
