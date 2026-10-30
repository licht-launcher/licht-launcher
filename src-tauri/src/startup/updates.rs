use std::sync::Arc;

use licht_core::{StartupPhase, StartupUpdateStatus};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use super::state::Session;

pub async fn run(app: &AppHandle, session: &Arc<Session>) -> anyhow::Result<bool> {
    let (manual, attempted) = match licht_core::read_startup_update_intent(env!(
        "CARGO_PKG_VERSION"
    )) {
        Ok(intent) => intent,
        Err(error) => {
            session.update(app, |state| state.warning = Some(format!("Could not read the saved update request: {error}. Starting the installed version.")))?;
            (false, false)
        }
    };
    let policy = licht_core::startup_update_policy(false, false, manual, attempted);
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
        let updater = app.updater_builder().configure_client(|client| client.connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT).read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)).build()?;
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
    let bytes = tokio::select! { bytes = download => bytes?, stopped = session.cancelled() => { stopped?; anyhow::bail!("startup closed"); } };
    if let Some(error) = progress_error
        .into_inner()
        .map_err(|_| anyhow::anyhow!("update progress lock failed"))?
    {
        anyhow::bail!(error);
    }
    // download() verifies the signed package before install() can run.
    licht_core::mark_startup_update_attempt(env!("CARGO_PKG_VERSION"))?;
    session.update(app, |state| state.phase = StartupPhase::InstallingUpdate)?;
    available.install(bytes)?;
    app.request_restart();
    Ok(true)
}

fn record_progress_error(slot: &std::sync::Mutex<Option<String>>, error: String) {
    match slot.lock() {
        Ok(mut slot) => *slot = Some(error),
        Err(_) => tracing::error!("Could not record update progress failure"),
    }
}
