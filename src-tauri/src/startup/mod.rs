use std::sync::Arc;

use anyhow::Context;
use licht_core::{StartupPhase, StartupReporter};
use tauri::{AppHandle, Manager};

pub mod commands;
pub mod preview;
pub mod state;
mod updates;

pub fn start(app: AppHandle, runtime: Arc<state::StartupRuntime>) -> anyhow::Result<()> {
    // The launcher window shows startup and update progress, then the launcher.
    crate::create_launcher_window(&app, "main")?;
    let session = runtime.session("main")?;
    let worker_runtime = Arc::clone(&runtime);
    let worker = tauri::async_runtime::spawn(async move {
        if let Err(error) = initialize(&app, &worker_runtime, &session).await {
            session.fail(&app, &format!("{error:#}"));
        }
    });
    runtime
        .workers
        .lock()
        .map_err(|_| anyhow::anyhow!("startup worker lock failed"))?
        .push(worker);
    Ok(())
}

async fn initialize(
    app: &AppHandle,
    runtime: &Arc<state::StartupRuntime>,
    session: &Arc<state::Session>,
) -> anyhow::Result<()> {
    session.wait_ready().await?;
    let preview = licht_core::take_startup_preview()?;
    if let Some(scenario) = preview {
        preview::run(app, session, scenario).await?;
    }
    let progress_app = app.clone();
    let progress_session = Arc::clone(session);
    let reporter = StartupReporter::new(move |progress| {
        progress_session.core_progress(&progress_app, progress)
    });
    let paths = crate::commands::LauncherPaths::load(reporter)
        .await
        .context("Could not prepare launcher storage")?;
    let settings = licht_core::load_settings(&paths.settings).await?;
    if preview.is_none()
        && updates::run(app, session, &paths.settings, settings.automatic_updates).await?
    {
        return Ok(());
    }
    session.update(app, |state| {
        state.phase = StartupPhase::StartingServices;
        state.total_bytes = None;
    })?;
    // The service closes the pool when the launcher shuts down.
    let database = licht_core::DatabaseService::start(paths.settings.clone()).await?;
    let database_client = database.client();
    *runtime.database.lock().await = Some(database);
    database_client.recover_download_jobs(&paths.game).await?;
    let client = reqwest::Client::builder()
        .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()?;
    let downloads = Arc::new(licht_core::DownloadManager::new(
        paths.settings.clone(),
        paths.game.clone(),
        crate::commands::host_environment()?,
        client,
        database_client.clone(),
    ));
    app.manage(database_client);
    app.manage(paths);
    app.manage(Arc::clone(&downloads));
    *runtime.downloads.lock().await = Some(Arc::clone(&downloads));
    crate::commands::downloads::forward_events(app.clone(), Arc::clone(&downloads));
    if session.is_stopped() {
        anyhow::bail!("startup closed");
    }
    downloads
        .recover_pending()
        .await
        .context("Could not restore the installation queue")?;
    if let Err(error) = crate::desktop::apply_preferences(app, &settings) {
        tracing::error!(%error, "Could not apply launcher behavior preferences");
        session.update(app, |state| {
            state.warning = Some(
                "Could not apply appearance preferences. Restart the launcher to try again."
                    .to_string(),
            )
        })?;
    }
    session.update(app, |state| state.phase = StartupPhase::Ready)?;
    if let Err(error) = crate::app_updates::start(
        app.clone(),
        Arc::clone(runtime),
        Arc::clone(session),
        settings.automatic_updates,
    ) {
        tracing::error!(
            error = %licht_core::error_chain(error.as_ref()),
            "Could not start background application updates"
        );
        session.update(app, |state| {
            state.warning = Some(
                "Background updates could not start. Restart the launcher to try again."
                    .to_string(),
            )
        })?;
    }
    Ok(())
}
