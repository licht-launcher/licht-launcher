use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::{command_error, command_failure};

pub const LABEL: &str = "settings";
const SECTION_EVENT: &str = "settings-window-section";
const LAUNCHER_CLOSED_EVENT: &str = "launcher-closed";
const LAUNCHER_CLOSED_GRACE: Duration = Duration::from_secs(5);
const MAX_SECTION_LENGTH: usize = 32;

/// Keeps a section requested while the page is still loading.
#[derive(Debug, Default)]
struct WindowState {
    section: Option<String>,
    ready: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Next {
    Create,
    Wait,
    Show,
}

impl WindowState {
    fn request(&mut self, section: &str, window_exists: bool) -> Next {
        self.section = Some(section.to_string());
        if !window_exists {
            self.ready = false;
            Next::Create
        } else if self.ready {
            Next::Show
        } else {
            Next::Wait
        }
    }

    fn mark_ready(&mut self) -> Option<String> {
        self.ready = true;
        self.section.clone()
    }

    fn closed(&mut self) {
        *self = Self::default();
    }
}

/// Held while creating the window, so two quick requests cannot both create it.
static STATE: Mutex<WindowState> = Mutex::new(WindowState {
    section: None,
    ready: false,
});

fn state() -> anyhow::Result<std::sync::MutexGuard<'static, WindowState>> {
    STATE
        .lock()
        .map_err(|_| anyhow::anyhow!("settings window lock failed"))
}

pub fn is_launcher_window(label: &str) -> bool {
    label == "main" || label == LABEL
}

fn valid_section(section: &str) -> bool {
    !section.is_empty()
        && section.len() <= MAX_SECTION_LENGTH
        && section.bytes().all(|byte| byte.is_ascii_lowercase())
}

pub fn open(app: &AppHandle, section: &str) -> anyhow::Result<()> {
    anyhow::ensure!(valid_section(section), "Unknown settings section");
    let mut state = state()?;
    let window = app.get_webview_window(LABEL);
    match (state.request(section, window.is_some()), window) {
        (Next::Show, Some(window)) => {
            app.emit_to(LABEL, SECTION_EVENT, section)?;
            window.unminimize()?;
            window.show()?;
            window.set_focus()?;
        }
        (Next::Create, _) => {
            let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
                .title("Settings - Licht Launcher")
                // The size the settings dialog had inside the launcher window.
                .inner_size(896.0, 664.0)
                .min_inner_size(760.0, 520.0)
                .maximizable(false)
                .center();
            crate::build_launcher_style_window(app, builder)?;
        }
        _ => {}
    }
    Ok(())
}

pub fn forget() {
    match state() {
        Ok(mut state) => state.closed(),
        Err(error) => tracing::error!(%error, "Could not reset the settings window state"),
    }
}

/// The page saves its last edit, then closes itself without being able to refuse.
pub fn close(app: &AppHandle) {
    close_after_last_edit(app, LABEL);
}

/// Asks the window `label` to save its last edit and close, and closes it if it does not.
pub(crate) fn close_after_last_edit(app: &AppHandle, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if let Err(error) = app.emit_to(label, LAUNCHER_CLOSED_EVENT, ()) {
        tracing::error!(%error, label, "Could not ask a settings window to close");
        destroy(&window);
        return;
    }
    let app = app.clone();
    let label = label.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(LAUNCHER_CLOSED_GRACE).await;
        if let Some(window) = app.get_webview_window(&label) {
            tracing::warn!(label, "A settings window did not close in time; closing it");
            destroy(&window);
        }
    });
}

pub(crate) fn destroy(window: &WebviewWindow) {
    if let Err(error) = window.destroy() {
        tracing::error!(%error, label = window.label(), "Could not close a settings window");
    }
}

#[tauri::command]
pub async fn open_settings_window(
    app: AppHandle,
    window: WebviewWindow,
    section: String,
) -> Result<(), String> {
    if !is_launcher_window(window.label()) {
        return Err("Settings can only be opened from the launcher".to_string());
    }
    // WebView2 creation must not block the synchronous IPC/UI thread on Windows.
    tauri::async_runtime::spawn_blocking(move || open(&app, &section))
        .await
        .map_err(|error| command_failure("Could not join settings window creation", &error))?
        .map_err(|error| command_error(error.as_ref()))
}

// Async: on the UI thread it could deadlock with `open` creating the window.
#[tauri::command]
pub async fn settings_window_ready(window: WebviewWindow) -> Result<Option<String>, String> {
    if window.label() != LABEL {
        return Err("Only the settings window can report that it is ready".to_string());
    }
    let mut state = state().map_err(|error| command_error(error.as_ref()))?;
    Ok(state.mark_ready())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_launcher_and_settings_windows_are_launcher_windows() {
        assert!(is_launcher_window("main"));
        assert!(is_launcher_window("settings"));
        assert!(!is_launcher_window("startup-preview-0"));
        assert!(!is_launcher_window(""));
    }

    #[test]
    fn accepts_only_short_lowercase_section_ids() {
        assert!(valid_section("account"));
        assert!(valid_section("troubleshooting"));
        assert!(!valid_section(""));
        assert!(!valid_section("Account"));
        assert!(!valid_section("java\"; alert(1); \""));
        assert!(!valid_section(&"a".repeat(MAX_SECTION_LENGTH + 1)));
    }

    #[test]
    fn creates_the_window_and_gives_the_page_the_requested_section() {
        let mut state = WindowState::default();
        assert_eq!(state.request("downloads", false), Next::Create);
        assert_eq!(state.mark_ready().as_deref(), Some("downloads"));
    }

    #[test]
    fn keeps_the_latest_section_requested_while_the_page_loads() {
        let mut state = WindowState::default();
        assert_eq!(state.request("downloads", false), Next::Create);
        assert_eq!(state.request("updater", true), Next::Wait);
        assert_eq!(state.mark_ready().as_deref(), Some("updater"));
    }

    #[test]
    fn sends_the_section_to_a_ready_page() {
        let mut state = WindowState::default();
        state.request("account", false);
        state.mark_ready();
        assert_eq!(state.request("java", true), Next::Show);
    }

    #[test]
    fn a_closed_window_is_created_again_and_waits_for_its_page() {
        let mut state = WindowState::default();
        state.request("account", false);
        state.mark_ready();
        state.closed();
        assert_eq!(state.request("java", false), Next::Create);
        assert_eq!(state.request("updater", true), Next::Wait);
    }

    #[test]
    fn a_window_closed_without_notice_is_created_again() {
        let mut state = WindowState::default();
        state.request("account", false);
        state.mark_ready();
        assert_eq!(state.request("java", false), Next::Create);
        assert_eq!(state.request("updater", true), Next::Wait);
    }

    #[test]
    fn open_command_requires_async_ipc_dispatch() {
        // WebView2 creation deadlocks on Windows when Tauri dispatches a synchronous command.
        fn require_async<F, Output>(_: F)
        where
            F: Fn(AppHandle, WebviewWindow, String) -> Output,
            Output: std::future::Future<Output = Result<(), String>>,
        {
        }
        require_async(open_settings_window);
    }
}
