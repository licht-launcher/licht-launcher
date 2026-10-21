use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use licht_core::{StartupPhase, StartupPreviewScenario, StartupReporter, StartupUpdateStatus};
use tauri::AppHandle;

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
    let label = format!(
        "startup-preview-{}",
        NEXT_PREVIEW.fetch_add(1, Ordering::SeqCst)
    );
    let session = Session::new(&label, true);
    runtime
        .sessions
        .lock()
        .map_err(|_| anyhow::anyhow!("startup sessions lock failed"))?
        .insert(label.clone(), Arc::clone(&session));
    create_window(&app, &label)?;
    let worker = tauri::async_runtime::spawn(async move {
        let result = async {
            session.wait_ready().await?;
            run(&app, &session, scenario).await
        }
        .await;
        if let Err(error) = result {
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
