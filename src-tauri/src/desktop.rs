use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Context;
use licht_core::{LauncherBehavior, Settings};
use tauri::{AppHandle, Manager, WebviewWindow};

pub struct Runtime {
    pub hardware_acceleration: bool,
    pub theme: Mutex<licht_core::LauncherTheme>,
    minimize_on_game_start: AtomicBool,
    restore_on_game_exit: AtomicBool,
    behavior: Mutex<LauncherBehavior>,
}

impl Runtime {
    pub fn new(hardware_acceleration: bool, theme: licht_core::LauncherTheme) -> Self {
        Self {
            hardware_acceleration,
            theme: Mutex::new(theme),
            minimize_on_game_start: AtomicBool::new(false),
            restore_on_game_exit: AtomicBool::new(false),
            behavior: Mutex::new(LauncherBehavior::default()),
        }
    }
}

pub fn restore(app: &AppHandle) -> anyhow::Result<()> {
    let window = app
        .get_webview_window("main")
        .context("Launcher window is unavailable")?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

pub fn apply_preferences(app: &AppHandle, settings: &Settings) -> anyhow::Result<()> {
    crate::theme::apply(app, settings.theme)?;
    let runtime = app.state::<Runtime>();
    runtime
        .minimize_on_game_start
        .store(settings.minimize_on_game_start, Ordering::SeqCst);
    runtime
        .restore_on_game_exit
        .store(settings.restore_on_game_exit, Ordering::SeqCst);
    Ok(())
}

pub fn game_started(app: &AppHandle, folder: &str) -> anyhow::Result<()> {
    let runtime = app.state::<Runtime>();
    // Keep window actions ordered with state changes from parallel launches.
    let mut behavior = runtime
        .behavior
        .lock()
        .map_err(|_| anyhow::anyhow!("Launcher behavior lock failed"))?;
    let minimize = behavior.game_started(
        folder,
        runtime.minimize_on_game_start.load(Ordering::SeqCst),
    );
    if minimize
        && let Some(window) = app.get_webview_window("main")
        && window.is_visible()?
    {
        window.minimize()?;
    }
    Ok(())
}

pub fn game_finished(app: &AppHandle, folder: &str) -> anyhow::Result<()> {
    let runtime = app.state::<Runtime>();
    let mut behavior = runtime
        .behavior
        .lock()
        .map_err(|_| anyhow::anyhow!("Launcher behavior lock failed"))?;
    let should_restore = behavior.game_finished(
        folder,
        runtime.minimize_on_game_start.load(Ordering::SeqCst),
        runtime.restore_on_game_exit.load(Ordering::SeqCst),
    );
    if should_restore {
        restore(app)?;
    }
    Ok(())
}

/// Extra WebView2 browser arguments for a new window, if any.
#[cfg(windows)]
pub fn webview_browser_args(app: &AppHandle) -> Option<String> {
    let hardware_acceleration = app.state::<Runtime>().hardware_acceleration;
    // msedgedriver passes its remote debugging port in this variable, but WebView2
    // did not apply it on the CI runner because the app (through wry) always sets
    // its own arguments. End-to-end builds forward it explicitly.
    #[cfg(feature = "e2e")]
    let driver = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").ok();
    #[cfg(not(feature = "e2e"))]
    let driver: Option<String> = None;
    browser_args(hardware_acceleration, driver.as_deref())
}

#[cfg(any(windows, test))]
fn browser_args(hardware_acceleration: bool, driver: Option<&str>) -> Option<String> {
    let mut args = Vec::new();
    if !hardware_acceleration {
        args.push("--disable-gpu");
    }
    if let Some(driver) = driver.map(str::trim).filter(|value| !value.is_empty()) {
        args.push(driver);
    }
    (!args.is_empty()).then(|| args.join(" "))
}

pub fn configure_webview(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
        let enabled = window.app_handle().state::<Runtime>().hardware_acceleration;
        window.with_webview(move |webview| {
            if let Some(settings) = webview.inner().settings() {
                settings.set_hardware_acceleration_policy(if enabled {
                    HardwareAccelerationPolicy::OnDemand
                } else {
                    HardwareAccelerationPolicy::Never
                });
            } else {
                tracing::error!(
                    "Could not access WebKitGTK settings to apply hardware acceleration"
                );
            }
        })?;
    }
    #[cfg(not(target_os = "linux"))]
    let _ = window;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::browser_args;

    #[test]
    fn hardware_acceleration_needs_no_extra_arguments() {
        assert_eq!(browser_args(true, None), None);
        assert_eq!(browser_args(true, Some("  ")), None);
    }

    #[test]
    fn disabled_acceleration_keeps_only_disable_gpu() {
        assert_eq!(browser_args(false, None).as_deref(), Some("--disable-gpu"));
    }

    #[test]
    fn webdriver_arguments_are_appended() {
        assert_eq!(
            browser_args(true, Some("--remote-debugging-port=0")).as_deref(),
            Some("--remote-debugging-port=0")
        );
        assert_eq!(
            browser_args(false, Some(" --remote-debugging-port=0 ")).as_deref(),
            Some("--disable-gpu --remote-debugging-port=0")
        );
    }
}
