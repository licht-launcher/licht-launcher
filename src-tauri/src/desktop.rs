use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Context;
use licht_core::{LauncherBehavior, Settings};
use tauri::{
    AppHandle, Manager, WebviewWindow,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};

const TRAY_ID: &str = "launcher";

pub struct Runtime {
    pub hardware_acceleration: bool,
    pub theme: Mutex<licht_core::LauncherTheme>,
    minimize_to_tray: AtomicBool,
    minimize_on_game_start: AtomicBool,
    restore_on_game_exit: AtomicBool,
    behavior: Mutex<LauncherBehavior>,
}

impl Runtime {
    pub fn new(hardware_acceleration: bool, theme: licht_core::LauncherTheme) -> Self {
        Self {
            hardware_acceleration,
            theme: Mutex::new(theme),
            minimize_to_tray: AtomicBool::new(false),
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

fn create_tray(app: &AppHandle) -> anyhow::Result<()> {
    let open = MenuItem::with_id(
        app,
        "launcher-open",
        "Open Licht Launcher",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "launcher-quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .context("Launcher icon is unavailable")?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Licht Launcher")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "launcher-open" => {
                if let Err(error) = restore(app) {
                    tracing::error!(%error, "Could not reopen the launcher from the system tray");
                }
            }
            "launcher-quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) && let Err(error) = restore(tray.app_handle())
            {
                tracing::error!(%error, "Could not reopen the launcher from the system tray");
            }
        })
        .build(app)?;
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
    if settings.minimize_to_tray {
        if app.tray_by_id(TRAY_ID).is_none() {
            create_tray(app)?;
        }
        runtime.minimize_to_tray.store(true, Ordering::SeqCst);
    } else {
        runtime.minimize_to_tray.store(false, Ordering::SeqCst);
        // Restore a hidden window before removing its only way back into the app.
        if app.tray_by_id(TRAY_ID).is_some() {
            if let Some(window) = app.get_webview_window("main")
                && !window.is_visible()?
            {
                restore(app)?;
            }
            app.remove_tray_by_id(TRAY_ID);
        }
    }
    Ok(())
}

pub fn hide_on_close(app: &AppHandle) -> bool {
    if !app
        .state::<Runtime>()
        .minimize_to_tray
        .load(Ordering::SeqCst)
    {
        return false;
    }
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };
    match window.hide() {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, "Could not hide the launcher in the system tray");
            false
        }
    }
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
