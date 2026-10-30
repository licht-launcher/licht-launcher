use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use licht_core::{StartupPhase, StartupPreviewScenario, StartupReporter, StartupUpdateStatus};
use tauri::{AppHandle, Emitter, Manager};

use super::{
    create_window,
    state::{Session, StartupRuntime},
};

static NEXT_PREVIEW: AtomicU64 = AtomicU64::new(0);

pub fn open(
    app: AppHandle,
    runtime: Arc<StartupRuntime>,
    scenario: StartupPreviewScenario,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        cfg!(debug_assertions),
        "Startup tests are only available in development builds"
    );
    // Lock worker registration before window creation, so setup cannot fail after opening it.
    let mut workers = runtime
        .workers
        .lock()
        .map_err(|_| anyhow::anyhow!("startup worker lock failed"))?;
    anyhow::ensure!(
        runtime.try_begin_preview(),
        "Close the current startup preview before opening another scenario"
    );
    let label = format!(
        "startup-preview-{}",
        NEXT_PREVIEW.fetch_add(1, Ordering::SeqCst)
    );
    let result = (|| -> anyhow::Result<()> {
        let session = Session::new(&label, true);
        runtime
            .sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("startup sessions lock failed"))?
            .insert(label.clone(), Arc::clone(&session));
        create_window(&app, &label)?;
        let worker_app = app.clone();
        let worker = tauri::async_runtime::spawn(async move {
            let result = async {
                session.wait_ready().await?;
                run(&worker_app, &session, scenario).await
            }
            .await;
            if let Err(error) = result {
                session.fail(&worker_app, &format!("{error:#}"));
            }
        });
        workers.push(worker);
        Ok(())
    })();
    drop(workers);
    if result.is_err() {
        runtime.finish_preview();
        if let Ok(mut sessions) = runtime.sessions.lock() {
            if let Some(session) = sessions.remove(&label) {
                session.stop();
            }
        } else {
            tracing::error!("Could not remove a failed startup preview session");
        }
    } else {
        publish_state(&app, runtime.preview_is_open());
    }
    result
}

pub fn close(app: &AppHandle) -> anyhow::Result<()> {
    for (label, window) in app.webview_windows() {
        if label.starts_with("startup-preview-") {
            window.close()?;
            break;
        }
    }
    Ok(())
}

pub fn publish_state(app: &AppHandle, open: bool) {
    if let Err(error) = app.emit("startup-preview-state", open) {
        tracing::warn!(%error, "Could not publish startup preview window state");
    }
}

pub async fn run(
    app: &AppHandle,
    session: &Arc<Session>,
    scenario: StartupPreviewScenario,
) -> anyhow::Result<()> {
    session.update(app, |state| {
        state.preview = true;
        state.phase = StartupPhase::CheckingUpdates;
    })?;
    pause(session).await?;
    if matches!(scenario, StartupPreviewScenario::CheckFailure) {
        session.update(app, |state| {
            state.update_status = StartupUpdateStatus::Failed;
            state.warning = Some(
                "Simulated network failure. The installed launcher remains available.".to_string(),
            );
        })?;
    } else {
        session.update(app, |state| {
            state.update_status = StartupUpdateStatus::UpToDate
        })?;
    }
    if matches!(
        scenario,
        StartupPreviewScenario::Update | StartupPreviewScenario::UpdateAndMigration
    ) {
        session.update(app, |state| {
            state.phase = StartupPhase::DownloadingUpdate;
            state.version = Some("preview-version".to_string());
            state.update_status = StartupUpdateStatus::Available;
            state.total_bytes = Some(10 * 1024 * 1024);
            state.processed_bytes = 0;
        })?;
        for step in 1..=10 {
            pause(session).await?;
            session.update(app, |state| state.processed_bytes = step * 1024 * 1024)?;
        }
        session.update(app, |state| {
            state.phase = StartupPhase::VerifyingUpdate;
            state.total_bytes = None;
        })?;
        pause(session).await?;
        session.update(app, |state| state.phase = StartupPhase::InstallingUpdate)?;
        pause(session).await?;
    }
    if matches!(
        scenario,
        StartupPreviewScenario::Migration
            | StartupPreviewScenario::UpdateAndMigration
            | StartupPreviewScenario::MigrationFailure
    ) {
        let session = Arc::clone(session);
        let app = app.clone();
        let reporter = StartupReporter::new(move |progress| session.core_progress(&app, progress));
        licht_core::run_startup_migration_preview(
            matches!(scenario, StartupPreviewScenario::MigrationFailure),
            reporter,
        )
        .await?;
    }
    session.update(app, |state| {
        state.phase = StartupPhase::Ready;
        state.total_bytes = None;
    })?;
    Ok(())
}

async fn pause(session: &Session) -> anyhow::Result<()> {
    tokio::select! { _ = tokio::time::sleep(Duration::from_millis(200)) => Ok(()), result = session.cancelled() => { result?; anyhow::bail!("startup preview closed"); } }
}
