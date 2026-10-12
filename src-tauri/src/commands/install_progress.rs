use licht_core::{DownloadProgress, InstallProgress};
use tauri::{AppHandle, Emitter};

pub(super) fn emit_install_progress(
    app: &AppHandle,
    progress_version_id: &str,
    progress: DownloadProgress,
) {
    let _ = app.emit(
        "install-progress",
        InstallProgress::tagged(progress_version_id, progress),
    );
}
