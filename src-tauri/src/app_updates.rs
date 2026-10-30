use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use licht_core::{AppUpdateCache, AppUpdatePhase, AppUpdateState};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard, watch};

use crate::startup::state::{Session, StartupRuntime};

const MAX_SAFE_BYTES: u64 = 9_007_199_254_740_991;

pub struct Runtime {
    state: Mutex<(AppUpdateState, Instant)>,
    operation: Arc<AsyncMutex<()>>,
    available: AsyncMutex<Option<Update>>,
    automatic: watch::Sender<bool>,
}

impl Runtime {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new((AppUpdateState::new(false), Instant::now())),
            operation: Arc::new(AsyncMutex::new(())),
            available: AsyncMutex::new(None),
            automatic: watch::channel(false).0,
        })
    }

    fn snapshot(&self) -> anyhow::Result<AppUpdateState> {
        Ok(self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("application update state lock failed"))?
            .0
            .clone())
    }

    fn publish(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut AppUpdateState),
    ) -> anyhow::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("application update state lock failed"))?;
        let previous = state.0.clone();
        change(&mut state.0);
        let force = previous.phase != state.0.phase
            || previous.version != state.0.version
            || previous.error != state.0.error
            || previous.automatic_updates != state.0.automatic_updates;
        if force || state.1.elapsed() >= Duration::from_millis(100) {
            app.emit("app-update-state", &state.0)?;
            state.1 = Instant::now();
        }
        Ok(())
    }

    pub fn preferences_changed(&self, app: &AppHandle, enabled: bool) -> anyhow::Result<()> {
        let changed = self.automatic.send_if_modified(|current| {
            if *current == enabled {
                return false;
            }
            *current = enabled;
            true
        });
        if changed {
            self.publish(app, |state| state.automatic_updates = enabled)?;
        }
        Ok(())
    }

    fn failed(&self, app: &AppHandle, message: &str) {
        if self
            .publish(app, |state| {
                state.phase = AppUpdatePhase::Failed;
                state.downloaded_bytes = 0;
                state.total_bytes = None;
                state.error = Some(message.to_string());
            })
            .is_err()
        {
            tracing::error!("Could not publish application update failure");
        }
    }

    fn reserve(&self) -> anyhow::Result<OwnedMutexGuard<()>> {
        Arc::clone(&self.operation)
            .try_lock_owned()
            .map_err(|_| anyhow::anyhow!("An update operation is already running"))
    }
}

fn cache(app: &AppHandle) -> anyhow::Result<AppUpdateCache> {
    let value = app
        .config()
        .plugins
        .0
        .get("updater")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Updater configuration is missing"))?;
    let config: tauri_plugin_updater::Config = serde_json::from_value(value)?;
    let directory = licht_core::SharedCache::system()?
        .root()
        .join("cache")
        .join("app-updates");
    Ok(AppUpdateCache::new(
        directory,
        env!("CARGO_PKG_VERSION"),
        &format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        &config.pubkey,
        config.require_signed_version,
    )?)
}

async fn load_cache(
    app: &AppHandle,
) -> anyhow::Result<Option<(licht_core::AppUpdatePackage, Vec<u8>)>> {
    let cache = cache(app)?;
    tauri::async_runtime::spawn_blocking(move || cache.load())
        .await?
        .map_err(Into::into)
}

async fn automatic_disabled(mut automatic: watch::Receiver<bool>) {
    loop {
        if !*automatic.borrow_and_update() {
            return;
        }
        if automatic.changed().await.is_err() {
            return;
        }
    }
}

fn updater(
    app: &AppHandle,
    startup: Arc<StartupRuntime>,
) -> anyhow::Result<tauri_plugin_updater::Updater> {
    let exit_app = app.clone();
    Ok(app
        .updater_builder()
        .configure_client(|client| {
            client
                .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
                .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        })
        .on_before_exit(move || {
            // install() runs on a blocking worker; Windows exits directly after this callback.
            tauri::async_runtime::block_on(startup.shutdown());
            exit_app.cleanup_before_exit();
        })
        .build()?)
}

async fn check(
    app: &AppHandle,
    runtime: &Arc<Runtime>,
    startup: Arc<StartupRuntime>,
    automatic: bool,
) -> anyhow::Result<()> {
    let previous = runtime.snapshot()?;
    runtime.publish(app, |state| {
        state.phase = AppUpdatePhase::Checking;
        state.downloaded_bytes = 0;
        state.total_bytes = None;
        state.error = None;
    })?;
    let updater = updater(app, startup)?;
    let result = if automatic {
        tokio::select! {
            result = updater.check() => Some(result),
            _ = automatic_disabled(runtime.automatic.subscribe()) => None,
        }
    } else {
        Some(updater.check().await)
    };
    let Some(result) = result else {
        runtime.publish(app, |state| {
            let enabled = state.automatic_updates;
            *state = previous;
            state.automatic_updates = enabled;
        })?;
        return Ok(());
    };
    let available = match result {
        Ok(available) => available,
        Err(_)
            if matches!(
                previous.phase,
                AppUpdatePhase::Cached | AppUpdatePhase::Ready
            ) =>
        {
            tracing::warn!(
                "Could not refresh application update metadata; keeping the verified cache"
            );
            runtime.publish(app, |state| {
                let automatic = state.automatic_updates;
                *state = previous;
                state.automatic_updates = automatic;
                state.error = Some(
                    "Could not refresh update metadata. The downloaded package is still cached."
                        .to_string(),
                );
            })?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let Some(available) = available else {
        let cache = cache(app)?;
        tauri::async_runtime::spawn_blocking(move || cache.clear()).await??;
        *runtime.available.lock().await = None;
        runtime.publish(app, |state| {
            state.phase = AppUpdatePhase::UpToDate;
            state.version = None;
            state.downloaded_bytes = 0;
            state.total_bytes = None;
            state.error = None;
        })?;
        return Ok(());
    };
    *runtime.available.lock().await = Some(available.clone());
    let cached = match load_cache(app).await {
        Ok(cached) => cached,
        Err(_) => {
            tracing::warn!("Discarding an invalid application update cache");
            let cache = cache(app)?;
            tauri::async_runtime::spawn_blocking(move || cache.clear()).await??;
            None
        }
    };
    if let Some((package, _)) = cached
        && package.matches_release(&available.version, &available.signature)
    {
        runtime.publish(app, |state| {
            state.phase = AppUpdatePhase::Ready;
            state.version = Some(package.version);
            state.downloaded_bytes = package.bytes;
            state.total_bytes = Some(package.bytes);
            state.error = None;
        })?;
        return Ok(());
    }
    runtime.publish(app, |state| {
        state.phase = AppUpdatePhase::Available;
        state.version = Some(available.version.clone());
        state.downloaded_bytes = 0;
        state.total_bytes = None;
    })?;
    let automatic_download = *runtime.automatic.borrow();
    if automatic_download {
        download(app, runtime, true).await?;
    }
    Ok(())
}

async fn download(app: &AppHandle, runtime: &Arc<Runtime>, automatic: bool) -> anyhow::Result<()> {
    let available = runtime
        .available
        .lock()
        .await
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Check for an update before downloading"))?;
    runtime.publish(app, |state| {
        state.phase = AppUpdatePhase::Downloading;
        state.version = Some(available.version.clone());
        state.downloaded_bytes = 0;
        state.total_bytes = None;
        state.error = None;
    })?;
    let request = available.download(
        |chunk, total| {
            if runtime
                .publish(app, |state| {
                    state.downloaded_bytes += chunk as u64;
                    state.total_bytes = total.filter(|value| *value <= MAX_SAFE_BYTES);
                })
                .is_err()
            {
                tracing::error!("Could not publish application update download progress");
            }
        },
        || {
            if runtime
                .publish(app, |state| state.phase = AppUpdatePhase::Verifying)
                .is_err()
            {
                tracing::error!("Could not publish application update verification progress");
            }
        },
    );
    let result = if automatic {
        tokio::select! {
            result = request => Some(result),
            _ = automatic_disabled(runtime.automatic.subscribe()) => None,
        }
    } else {
        Some(request.await)
    };
    let Some(result) = result else {
        runtime.publish(app, |state| {
            state.phase = AppUpdatePhase::Available;
            state.downloaded_bytes = 0;
            state.total_bytes = None;
        })?;
        return Ok(());
    };
    let bytes = result?;
    let cache = cache(app)?;
    let package = tauri::async_runtime::spawn_blocking(move || {
        cache.store(&available.version, &available.signature, &bytes)
    })
    .await??;
    runtime.publish(app, |state| {
        state.phase = AppUpdatePhase::Ready;
        state.version = Some(package.version);
        state.downloaded_bytes = package.bytes;
        state.total_bytes = Some(package.bytes);
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn automatic_download_cancellation_observes_the_current_preference() {
        let runtime = Runtime::new();
        automatic_disabled(runtime.automatic.subscribe()).await;
        runtime.automatic.send_replace(true);
        let waiting = tokio::spawn(automatic_disabled(runtime.automatic.subscribe()));
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        runtime.automatic.send_replace(false);
        tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("cancellation")
            .expect("watcher");
    }

    #[test]
    fn native_update_operations_are_serialized_without_queueing_duplicates() {
        let runtime = Runtime::new();
        let first = runtime.reserve().expect("first operation");
        assert!(runtime.reserve().is_err());
        drop(first);
        assert!(runtime.reserve().is_ok());
    }
}

pub fn start(
    app: AppHandle,
    startup: Arc<StartupRuntime>,
    session: Arc<Session>,
    automatic: bool,
) -> anyhow::Result<()> {
    let runtime = Arc::clone(app.state::<Arc<Runtime>>().inner());
    runtime.preferences_changed(&app, automatic)?;
    let initial_operation = runtime.reserve()?;
    let mut workers = startup
        .workers
        .lock()
        .map_err(|_| anyhow::anyhow!("startup worker lock failed"))?;
    let startup_worker = Arc::clone(&startup);
    let worker = tauri::async_runtime::spawn(async move {
        let work = async {
            runtime.publish(&app, |state| state.phase = AppUpdatePhase::Verifying)?;
            match load_cache(&app).await {
                Ok(Some((package, _))) => {
                    runtime.publish(&app, |state| {
                        state.phase = AppUpdatePhase::Cached;
                        state.version = Some(package.version);
                        state.downloaded_bytes = package.bytes;
                        state.total_bytes = Some(package.bytes);
                    })?;
                }
                Ok(None) => {
                    runtime.publish(&app, |state| state.phase = AppUpdatePhase::Idle)?;
                }
                Err(_) => runtime.failed(
                    &app,
                    "The cached update could not be verified. Check for updates to try again.",
                ),
            }
            drop(initial_operation);
            let mut preferences = runtime.automatic.subscribe();
            loop {
                let automatic = *preferences.borrow_and_update();
                if automatic
                    && licht_core::app_update_can_check(runtime.snapshot()?.phase)
                    && let Ok(_operation) = runtime.reserve()
                    && check(&app, &runtime, Arc::clone(&startup_worker), true)
                        .await
                        .is_err()
                {
                    runtime.failed(&app, "Could not prepare an automatic update. Check your connection and try again.");
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(licht_core::APP_UPDATE_INTERVAL_SECONDS)) => {},
                    changed = preferences.changed() => {
                        if changed.is_err() { return Err::<(), anyhow::Error>(anyhow::anyhow!("update preferences channel closed")); }
                    },
                }
            }
        };
        tokio::select! {
            result = work => { if result.is_err() { runtime.failed(&app, "Background updates stopped. Restart the launcher to try again."); } },
            result = session.cancelled() => { if result.is_err() { tracing::error!("Could not observe update worker shutdown"); } },
        }
    });
    workers.push(worker);
    Ok(())
}

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Updates must be requested from the launcher".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn get_app_update_state(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<AppUpdateState, String> {
    require_main(&window)?;
    runtime
        .snapshot()
        .map_err(|_| "Could not read application update state".to_string())
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    startup: State<'_, Arc<StartupRuntime>>,
) -> Result<(), String> {
    require_main(&window)?;
    let runtime = Arc::clone(runtime.inner());
    let startup = Arc::clone(startup.inner());
    let operation = runtime.reserve().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn(async move {
        let _operation = operation;
        if check(&app, &runtime, startup, false).await.is_err() {
            runtime.failed(
                &app,
                "Could not check for updates. Check your connection and try again.",
            );
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn download_app_update(
    app: AppHandle,
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    require_main(&window)?;
    let runtime = Arc::clone(runtime.inner());
    let operation = runtime.reserve().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn(async move {
        let _operation = operation;
        if download(&app, &runtime, false).await.is_err() {
            runtime.failed(&app, "Could not download and verify the update. Try again.");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    downloads: State<'_, Arc<licht_core::DownloadManager>>,
) -> Result<(), String> {
    require_main(&window)?;
    let _operation = runtime.reserve().map_err(|error| error.to_string())?;
    let mut verified = false;
    let result = async {
        anyhow::ensure!(
            runtime.snapshot()?.phase == AppUpdatePhase::Ready,
            "Download and verify the update before installing it"
        );
        let available = runtime.available.lock().await.clone().ok_or_else(|| {
            anyhow::anyhow!("Check for updates before installing the cached package")
        })?;
        let (package, bytes) = load_cache(&app)
            .await?
            .ok_or_else(|| anyhow::anyhow!("The cached update is missing"))?;
        anyhow::ensure!(
            package.matches_release(&available.version, &available.signature),
            "The cached update changed. Check for updates again"
        );
        verified = true;
        downloads.suspend_all().await?;
        licht_core::mark_startup_update_attempt(env!("CARGO_PKG_VERSION"))?;
        runtime.publish(&app, |state| {
            state.phase = AppUpdatePhase::Installing;
            state.error = None;
        })?;
        tauri::async_runtime::spawn_blocking(move || available.install(bytes)).await??;
        app.request_restart();
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if result.is_err() {
        runtime
            .publish(&app, |state| {
                state.phase = if verified {
                    AppUpdatePhase::Ready
                } else {
                    AppUpdatePhase::Failed
                };
                state.error = Some(
                    "Could not install the update. You can retry or check for updates again."
                        .to_string(),
                );
            })
            .map_err(|_| "Could not publish application update failure".to_string())?;
    }
    result.map_err(|_| "Could not install the cached update. Try again.".to_string())
}
