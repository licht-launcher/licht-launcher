use std::sync::Arc;

use licht_core::{Database, StartupPhase, StartupUpdatePolicy, StartupUpdateStatus};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use super::state::Session;

/// Whether startup checks for, downloads and installs an update before the launcher opens.
fn startup_policy(
    self_updates: bool,
    release_build: bool,
    automatic: bool,
    manual: bool,
    attempted: bool,
) -> StartupUpdatePolicy {
    if !self_updates {
        return StartupUpdatePolicy {
            check: false,
            install: false,
            blocked: false,
        };
    }
    // Development builds would otherwise replace themselves with the published release.
    let automatic = automatic && release_build;
    licht_core::startup_update_policy(automatic, automatic, manual, attempted)
}

/// Returns true when an update was installed and the launcher is restarting.
pub async fn run(
    app: &AppHandle,
    session: &Arc<Session>,
    database: &Database,
    automatic: bool,
) -> anyhow::Result<bool> {
    let self_updates = app
        .state::<Arc<crate::app_updates::Runtime>>()
        .self_updates_enabled()?;
    let (manual, attempted) = match licht_core::read_startup_update_intent(env!(
        "CARGO_PKG_VERSION"
    )) {
        Ok(intent) => intent,
        Err(error) => {
            session.update(app, |state| state.warning = Some(format!("Could not read the saved update request: {error}. Starting the installed version.")))?;
            (false, false)
        }
    };
    let policy = startup_policy(
        self_updates,
        !cfg!(debug_assertions),
        automatic,
        manual,
        attempted,
    );
    if policy.blocked {
        session.update(app, |state| {
            state.update_status = StartupUpdateStatus::Failed;
            state.warning =
                Some("The previous update did not finish. Retry from Updates.".to_string());
        })?;
        return Ok(false);
    }
    if !policy.check {
        return Ok(false);
    }
    session.update(app, |state| state.phase = StartupPhase::CheckingUpdates)?;
    let result = async {
        let database = database.clone();
        let updater = app
            .updater_builder()
            .configure_client(|client| {
                client
                    .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
                    .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
            })
            .on_before_exit(move || tauri::async_runtime::block_on(database.close()))
            .build()?;
        tokio::select! { result = updater.check() => Ok::<_, anyhow::Error>(result?), stopped = session.cancelled() => { stopped?; anyhow::bail!("startup closed"); } }
    }.await;
    let available = match result {
        Ok(available) => available,
        Err(error) if !session.is_stopped() => {
            session.update(app, |state| {
                state.update_status = StartupUpdateStatus::Failed;
                state.warning = Some(format!(
                    "Could not check for updates. Starting the installed version: {error}"
                ));
            })?;
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    let Some(available) = available else {
        licht_core::clear_startup_update_request()?;
        session.update(app, |state| {
            state.update_status = StartupUpdateStatus::UpToDate
        })?;
        return Ok(false);
    };
    session.update(app, |state| {
        state.update_status = StartupUpdateStatus::Available;
        state.version = Some(available.version.clone());
    })?;
    if !policy.install {
        return Ok(false);
    }
    session.update(app, |state| {
        state.phase = StartupPhase::DownloadingUpdate;
        state.processed_bytes = 0;
        state.total_bytes = None;
    })?;
    let progress_error = std::sync::Mutex::new(None);
    let download = available.download(
        |chunk, total| {
            if let Err(error) = session.update(app, |state| {
                state.processed_bytes += chunk as u64;
                state.total_bytes = total;
            }) {
                record_progress_error(&progress_error, error.to_string());
            }
        },
        || {
            if let Err(error) =
                session.update(app, |state| state.phase = StartupPhase::VerifyingUpdate)
            {
                record_progress_error(&progress_error, error.to_string());
            }
        },
    );
    let bytes = tokio::select! {
        bytes = download => match bytes {
            Ok(bytes) => bytes,
            // An unreachable or invalid package must not keep the installed version from opening.
            Err(error) => {
                session.update(app, |state| {
                    state.update_status = StartupUpdateStatus::Failed;
                    state.warning = Some(format!(
                        "Could not download the update. Starting the installed version: {error}"
                    ));
                })?;
                return Ok(false);
            }
        },
        stopped = session.cancelled() => { stopped?; anyhow::bail!("startup closed"); }
    };
    if let Some(error) = progress_error
        .into_inner()
        .map_err(|_| anyhow::anyhow!("update progress lock failed"))?
    {
        anyhow::bail!(error);
    }
    // download() verifies the signed package before install() can run.
    licht_core::mark_startup_update_attempt(env!("CARGO_PKG_VERSION"))?;
    session.update(app, |state| state.phase = StartupPhase::InstallingUpdate)?;
    // Windows exits inside install() after on_before_exit, which blocks on the
    // database, so install() must not run on an async worker.
    tauri::async_runtime::spawn_blocking(move || available.install(bytes)).await??;
    app.request_restart();
    Ok(true)
}

fn record_progress_error(slot: &std::sync::Mutex<Option<String>>, error: String) {
    match slot.lock() {
        Ok(mut slot) => *slot = Some(error),
        Err(_) => tracing::error!("Could not record update progress failure"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_updates_install_before_the_launcher_opens() {
        let policy = startup_policy(true, true, true, false, false);
        assert!(policy.check && policy.install && !policy.blocked);
    }

    #[test]
    fn turning_automatic_updates_off_skips_the_startup_check() {
        let policy = startup_policy(true, true, false, false, false);
        assert!(!policy.check && !policy.install);
        // A restart the user asked for still updates.
        let policy = startup_policy(true, true, false, true, false);
        assert!(policy.check && policy.install);
    }

    #[test]
    fn development_builds_only_update_when_asked() {
        let policy = startup_policy(true, false, true, false, false);
        assert!(!policy.check && !policy.install);
    }

    #[test]
    fn installations_that_do_not_update_themselves_never_update_at_startup() {
        let policy = startup_policy(false, true, true, true, true);
        assert!(!policy.check && !policy.install && !policy.blocked);
    }

    #[test]
    fn an_unfinished_update_blocks_the_next_attempt() {
        assert!(startup_policy(true, true, true, false, true).blocked);
    }
}
