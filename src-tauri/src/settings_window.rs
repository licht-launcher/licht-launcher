//! The launcher settings open in their own window beside the launcher window.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::{command_error, command_failure};

pub const LABEL: &str = "settings";
/// Sent to an open settings window when another section is requested.
const SECTION_EVENT: &str = "settings-window-section";
const MAX_SECTION_LENGTH: usize = 32;

/// Windows that may use launcher-only commands such as updates and diagnostics.
pub fn is_launcher_window(label: &str) -> bool {
    label == "main" || label == LABEL
}

/// Section ids are lowercase words; the page falls back to its first section for
/// one it does not know.
fn valid_section(section: &str) -> bool {
    !section.is_empty()
        && section.len() <= MAX_SECTION_LENGTH
        && section.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn initialization_script(section: &str) -> Result<String, serde_json::Error> {
    Ok(format!(
        "window.__LICHT_SETTINGS_SECTION__ = {};",
        serde_json::to_string(section)?
    ))
}

/// Shows the settings window at `section`, creating it when it is not open yet.
pub fn open(app: &AppHandle, section: &str) -> anyhow::Result<()> {
    anyhow::ensure!(valid_section(section), "Unknown settings section");
    if let Some(window) = app.get_webview_window(LABEL) {
        app.emit_to(LABEL, SECTION_EVENT, section)?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("Settings - Licht Launcher")
        // The size the settings dialog had inside the launcher window.
        .inner_size(896.0, 664.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .initialization_script(initialization_script(section)?);
    crate::build_launcher_style_window(app, builder)?;
    Ok(())
}

/// Closes the settings window, so closing the launcher window ends the app.
pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL)
        && let Err(error) = window.close()
    {
        tracing::error!(%error, "Could not close the settings window");
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
    fn passes_the_section_to_the_page_as_a_json_string() {
        assert_eq!(
            initialization_script("downloads").expect("script"),
            "window.__LICHT_SETTINGS_SECTION__ = \"downloads\";"
        );
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
