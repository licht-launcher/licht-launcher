//! Tauri command handlers. Thin wrappers over `licht-core`.

pub mod accounts;
pub mod downloads;
pub mod frontend_log;
mod install_progress;
pub mod instances;
pub mod launch;
pub mod modpacks;
mod sessions;
pub mod settings;
pub mod shell;
pub mod versions;

pub use sessions::LaunchSessions;

use std::collections::BTreeMap;
use std::path::PathBuf;

use licht_core::{
    Arch, CoreError, LaunchEnvironment, MANIFEST_REQUEST_TIMEOUT, OsName, SharedCache, game_cache,
};

/// Folders chosen when the process started. A saved data directory is used on the
/// next start, so this process keeps the folders it opened with.
#[derive(Clone)]
pub struct LauncherPaths {
    /// The profile's database pool; its cache is the settings folder.
    pub settings: licht_core::Database,
    pub game: SharedCache,
}

impl LauncherPaths {
    pub async fn load(reporter: licht_core::StartupReporter) -> Result<Self, CoreError> {
        let settings = licht_core::initialize_launcher_with_progress(reporter).await?;
        let game = game_cache(&settings).await?;
        Ok(Self { settings, game })
    }
}

/// Shared HTTP client with a per-request timeout (avoids hanging forever on Mojang).
pub(crate) fn http_client() -> Result<reqwest::Client, CoreError> {
    reqwest::Client::builder()
        .timeout(MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)
}

/// Logs a failed command with every cause, then returns the message the interface shows.
pub(crate) fn failure(error: CoreError) -> String {
    command_error(&error)
}

/// Like [`failure`], for errors that do not come from the core.
pub(crate) fn command_error(error: &dyn std::error::Error) -> String {
    tracing::warn!(error = %licht_core::error_chain(error), "Command failed");
    error.to_string()
}

/// Logs `summary` with the cause behind it and returns only `summary` to the interface.
pub(crate) fn command_failure(summary: &str, error: &dyn std::error::Error) -> String {
    tracing::warn!(
        error = %format!("{summary}: {}", licht_core::error_chain(error)),
        "Command failed"
    );
    summary.to_string()
}

pub(crate) const SETTINGS_CHANGED_EVENT: &str = "settings-changed";
pub(crate) const ACCOUNTS_CHANGED_EVENT: &str = "accounts-changed";

/// The change already succeeded, so a failed notification is only logged.
pub(crate) fn notify_windows(app: &tauri::AppHandle, event: &str) {
    use tauri::Emitter;
    if let Err(error) = app.emit(event, ()) {
        tracing::error!(%error, event, "Could not notify windows of a change");
    }
}

pub(crate) fn host_environment() -> Result<LaunchEnvironment, CoreError> {
    let os = match std::env::consts::OS {
        "windows" => OsName::Windows,
        "linux" => OsName::Linux,
        "macos" => OsName::Osx,
        _ => return Err(CoreError::LaunchHost),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "x86" => Arch::X86,
        _ => return Err(CoreError::LaunchHost),
    };
    Ok(LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    })
}

pub(crate) fn instances_root(paths: &LauncherPaths) -> Result<PathBuf, CoreError> {
    paths.game.instances_dir()
}

#[cfg(test)]
mod tests {
    use std::io;

    use licht_core::CoreError;
    use licht_testkit::logs::LogCapture;

    use super::{command_error, command_failure, failure};

    fn nested_error() -> CoreError {
        let cause = io::Error::new(io::ErrorKind::PermissionDenied, "the folder is read-only");
        CoreError::ModrinthLegacyIndex(Box::new(CoreError::from(cause)))
    }

    #[test]
    fn failures_keep_the_message_shown_to_the_interface() {
        let message = failure(nested_error());

        assert_eq!(message, nested_error().to_string());
    }

    #[test]
    fn failures_are_logged_as_warnings_with_every_cause() {
        let capture = LogCapture::start();
        failure(nested_error());
        let logs = capture.text();

        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(logs.contains("Command failed"), "unexpected logs: {logs}");
        assert!(
            logs.contains(&format!(
                "{}: I/O operation failed: the folder is read-only",
                nested_error()
            )),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn command_errors_keep_their_message_and_log_every_cause() {
        let error = io::Error::other(nested_error());

        let capture = LogCapture::start();
        let message = command_error(&error);
        let logs = capture.text();

        assert_eq!(message, error.to_string());
        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(logs.contains("Command failed"), "unexpected logs: {logs}");
        assert!(
            logs.contains(&format!(
                "{}: I/O operation failed: the folder is read-only",
                nested_error()
            )),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn command_failures_show_their_summary_and_log_the_cause_behind_it() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "the worker stopped");

        let capture = LogCapture::start();
        let message = command_failure("Java check failed", &error);
        let logs = capture.text();

        assert_eq!(message, "Java check failed");
        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(
            logs.contains("Java check failed: the worker stopped"),
            "unexpected logs: {logs}"
        );
    }
}
