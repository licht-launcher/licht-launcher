use licht_core::LauncherTheme;
use tauri::{AppHandle, Emitter, Manager, Theme};

pub fn native_theme(theme: LauncherTheme) -> Option<Theme> {
    match theme {
        LauncherTheme::Light => Some(Theme::Light),
        LauncherTheme::Dark => Some(Theme::Dark),
        LauncherTheme::System => None,
    }
}

pub fn preference(app: &AppHandle) -> tauri::Result<LauncherTheme> {
    app.state::<crate::desktop::Runtime>()
        .theme
        .lock()
        .map(|theme| *theme)
        .map_err(|_| tauri::Error::Anyhow(anyhow::anyhow!("Launcher theme lock failed")))
}

pub fn initialization_script(theme: LauncherTheme) -> tauri::Result<String> {
    Ok(format!(
        "window.__LICHT_THEME__ = {};",
        serde_json::to_string(&theme)?
    ))
}

pub fn apply(app: &AppHandle, theme: LauncherTheme) -> tauri::Result<()> {
    let runtime = app.state::<crate::desktop::Runtime>();
    let mut current = runtime
        .theme
        .lock()
        .map_err(|_| tauri::Error::Anyhow(anyhow::anyhow!("Launcher theme lock failed")))?;
    if *current == theme {
        return Ok(());
    }
    for window in app.webview_windows().values() {
        window.set_theme(native_theme(theme))?;
    }
    app.set_theme(native_theme(theme));
    *current = theme;
    app.emit("launcher-theme-changed", theme)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_leaves_native_theme_selection_to_the_os() {
        assert_eq!(native_theme(LauncherTheme::Light), Some(Theme::Light));
        assert_eq!(native_theme(LauncherTheme::Dark), Some(Theme::Dark));
        assert_eq!(native_theme(LauncherTheme::System), None);
    }

    #[test]
    fn bootstrap_preserves_the_saved_preference_for_each_window() {
        assert_eq!(
            initialization_script(LauncherTheme::System).expect("script"),
            "window.__LICHT_THEME__ = \"system\";"
        );
    }
}
