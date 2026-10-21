use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use licht_core::{StartupPhase, StartupProgress};
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

pub struct Session {
    pub label: String,
    progress: Mutex<(StartupProgress, Instant)>,
    ready: watch::Sender<bool>,
    stopped: watch::Sender<bool>,
}

impl Session {
    pub fn new(label: &str, preview: bool) -> Arc<Self> {
        Arc::new(Self {
            label: label.to_string(),
            progress: Mutex::new((
                StartupProgress {
                    preview,
                    ..Default::default()
                },
                Instant::now(),
            )),
            ready: watch::channel(false).0,
            stopped: watch::channel(false).0,
        })
    }

    pub fn snapshot(&self) -> licht_core::Result<StartupProgress> {
        Ok(self
            .progress
            .lock()
            .map_err(|_| std::io::Error::other("startup status lock failed"))?
            .0
            .clone())
    }

    pub fn acknowledge(&self) {
        self.ready.send_replace(true);
    }
    pub fn stop(&self) {
        self.stopped.send_replace(true);
    }
    pub fn is_stopped(&self) -> bool {
        *self.stopped.borrow()
    }

    pub async fn wait_ready(&self) -> anyhow::Result<()> {
        let mut ready = self.ready.subscribe();
        tokio::select! {
            result = ready.wait_for(|value| *value) => { result.map_err(|_| anyhow::anyhow!("startup readiness channel closed"))?; Ok(()) },
            result = self.cancelled() => { result?; anyhow::bail!("startup closed") },
        }
    }

    pub async fn cancelled(&self) -> anyhow::Result<()> {
        let mut stopped = self.stopped.subscribe();
        stopped
            .wait_for(|value| *value)
            .await
            .map_err(|_| anyhow::anyhow!("startup cancellation channel closed"))?;
        Ok(())
    }

    pub fn update(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut StartupProgress),
    ) -> licht_core::Result<()> {
        if self.is_stopped() {
            return Err(std::io::Error::other("startup closed").into());
        }
        let snapshot = {
            let mut state = self
                .progress
                .lock()
                .map_err(|_| std::io::Error::other("startup status lock failed"))?;
            let previous = state.0.clone();
            change(&mut state.0);
            let force = previous.phase != state.0.phase
                || previous.error != state.0.error
                || previous.warning != state.0.warning
                || previous.update_status != state.0.update_status;
            if !force && state.1.elapsed() < Duration::from_millis(100) {
                return Ok(());
            }
            state.1 = Instant::now();
            state.0.clone()
        };
        if self.label == "startup" {
            app.emit("startup-progress", &snapshot)
        } else {
            app.emit_to(self.label.as_str(), "startup-progress", &snapshot)
        }
        .map_err(std::io::Error::other)?;
        Ok(())
    }

    pub fn core_progress(
        &self,
        app: &AppHandle,
        incoming: StartupProgress,
    ) -> licht_core::Result<()> {
        self.update(app, |state| {
            state.phase = incoming.phase;
            state.processed_files = incoming.processed_files;
            state.processed_bytes = incoming.processed_bytes;
            state.total_bytes = incoming.total_bytes;
        })
    }

    pub fn fail(&self, app: &AppHandle, error: &str) {
        if !self.is_stopped()
            && self
                .update(app, |state| {
                    state.phase = StartupPhase::Failed;
                    state.error = Some(error.to_string());
                })
                .is_err()
        {
            tracing::error!("Could not publish startup failure");
        }
    }
}

pub struct StartupRuntime {
    pub shutdown_complete: std::sync::atomic::AtomicBool,
    pub sessions: Mutex<HashMap<String, Arc<Session>>>,
    pub database: tokio::sync::Mutex<Option<licht_core::DatabaseService>>,
    pub downloads: tokio::sync::Mutex<Option<Arc<licht_core::DownloadManager>>>,
    pub workers: Mutex<Vec<tauri::async_runtime::JoinHandle<()>>>,
}

impl StartupRuntime {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            shutdown_complete: std::sync::atomic::AtomicBool::new(false),
            sessions: Mutex::new(HashMap::from([(
                "startup".to_string(),
                Session::new("startup", false),
            )])),
            database: tokio::sync::Mutex::new(None),
            downloads: tokio::sync::Mutex::new(None),
            workers: Mutex::new(Vec::new()),
        })
    }

    pub fn session(&self, label: &str) -> licht_core::Result<Arc<Session>> {
        let label = if label == "main" { "startup" } else { label };
        self.sessions
            .lock()
            .map_err(|_| std::io::Error::other("startup sessions lock failed"))?
            .get(label)
            .cloned()
            .ok_or_else(|| std::io::Error::other("startup session not found").into())
    }

    pub async fn shutdown(&self) {
        if let Ok(sessions) = self.sessions.lock() {
            for session in sessions.values() {
                session.stop();
            }
        } else {
            tracing::error!("Could not signal startup shutdown");
        }
        let workers = match self.workers.lock() {
            Ok(mut workers) => std::mem::take(&mut *workers),
            Err(_) => {
                tracing::error!("Could not join startup workers");
                Vec::new()
            }
        };
        for worker in workers {
            if worker.await.is_err() {
                tracing::error!("Startup worker join failed");
            }
        }
        if let Some(downloads) = self.downloads.lock().await.take()
            && downloads.suspend_all().await.is_err()
        {
            tracing::error!("Failed to suspend downloads during shutdown");
        }
        if let Some(database) = self.database.lock().await.take()
            && database.shutdown().await.is_err()
        {
            tracing::error!("Failed to close database during shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn readiness_and_stop_signals_survive_subscription_order() {
        let ready = Session::new("startup", false);
        ready.acknowledge();
        ready.wait_ready().await.expect("already acknowledged");
        ready.stop();
        ready.cancelled().await.expect("already stopped");
        assert!(ready.is_stopped());
        let closed = Session::new("startup-preview-1", true);
        closed.stop();
        assert!(closed.wait_ready().await.is_err());
        assert!(closed.snapshot().expect("snapshot").preview);
    }

    #[tokio::test]
    async fn shutdown_stops_root_and_preview_sessions_before_services_exist() {
        let runtime = StartupRuntime::new();
        let root = runtime
            .session("main")
            .expect("main uses root startup status");
        let preview = Session::new("startup-preview-1", true);
        runtime
            .sessions
            .lock()
            .expect("sessions")
            .insert(preview.label.clone(), Arc::clone(&preview));
        assert!(!root.is_stopped() && !preview.is_stopped());
        runtime.shutdown().await;
        assert!(root.is_stopped() && preview.is_stopped());
        assert!(runtime.database.lock().await.is_none());
        assert!(runtime.downloads.lock().await.is_none());
    }
}
