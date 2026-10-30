use std::sync::Arc;

use anyhow::Context;
use licht_core::{StartupPhase, StartupReporter};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub mod commands;
pub mod preview;
pub mod state;
mod updates;

pub fn create_window(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let theme = crate::theme::preference(app)?;
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title("Licht Launcher")
        .inner_size(420.0, 320.0)
        .center()
        .decorations(false)
        .visible(false)
        .resizable(false)
        .maximizable(false)
        .theme(crate::theme::native_theme(theme))
        .initialization_script(crate::theme::initialization_script(theme)?);
    #[cfg(windows)]
    let builder = if app.state::<crate::desktop::Runtime>().hardware_acceleration {
        builder
    } else {
        builder.additional_browser_args("--disable-gpu")
    };
    let window = builder.build()?;
    crate::desktop::configure_webview(&window)?;
    Ok(())
}

pub fn start(app: AppHandle, runtime: Arc<state::StartupRuntime>) -> anyhow::Result<()> {
    create_window(&app, "startup")?;
    let session = runtime.session("startup")?;
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
    if let Some(scenario) = licht_core::take_startup_preview()? {
        preview::run(app, session, scenario).await?;
    } else if updates::run(app, session).await? {
        return Ok(());
    }
    let progress_app = app.clone();
    let progress_session = Arc::clone(session);
    let reporter = StartupReporter::new(move |progress| {
        progress_session.core_progress(&progress_app, progress)
    });
    let paths = crate::commands::LauncherPaths::load(reporter)
        .await
        .context("Could not prepare launcher storage")?;
    session.update(app, |state| {
        state.phase = StartupPhase::StartingServices;
        state.total_bytes = None;
    })?;
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
    let settings = licht_core::load_settings(&paths.settings)?;
    let custom_title_bar = !settings.disable_custom_title_bar;
    app.manage(database_client);
    app.manage(paths);
    app.manage(Arc::clone(&downloads));
    app.manage(crate::commands::shell::TitleBarMode {
        custom: custom_title_bar,
    });
    *runtime.downloads.lock().await = Some(Arc::clone(&downloads));
    crate::commands::downloads::forward_events(app.clone(), Arc::clone(&downloads));
    if session.is_stopped() {
        anyhow::bail!("startup closed");
    }
    downloads
        .recover_pending()
        .await
        .context("Could not restore the installation queue")?;
    crate::create_main_window(app, custom_title_bar, !settings.disable_transparency)?;
    if let Err(error) = crate::desktop::apply_preferences(app, &settings) {
        tracing::error!(%error, "Could not apply launcher behavior preferences");
        session.update(app, |state| {
            state.warning = Some(
                "System tray setup failed. Closing the window will exit the launcher.".to_string(),
            )
        })?;
    }
    session.update(app, |state| state.phase = StartupPhase::Ready)?;
    if crate::app_updates::start(
        app.clone(),
        Arc::clone(runtime),
        Arc::clone(session),
        settings.automatic_updates,
    )
    .is_err()
    {
        tracing::error!("Could not start background application updates");
        session.update(app, |state| {
            state.warning = Some(
                "Background updates could not start. Restart the launcher to try again."
                    .to_string(),
            )
        })?;
    }
    if let Some(window) = app.get_webview_window("startup") {
        window.close()?;
    }
    Ok(())
}
