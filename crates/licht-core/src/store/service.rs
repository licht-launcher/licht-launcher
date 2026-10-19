use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use tokio::sync::{mpsc, oneshot};

use super::db::{self, META_LAST_PLAYED};
use crate::{CoreError, Result, SharedCache};

const QUEUE_CAPACITY: usize = 64;

enum Request {
    LastPlayed(oneshot::Sender<Result<Option<String>>>),
    SetLastPlayed(String, oneshot::Sender<Result<()>>),
    ClearLastPlayed(oneshot::Sender<Result<()>>),
    Stop,
}

/// Owns the database thread. Explicit shutdown waits for accepted work and connection closure.
#[derive(Debug)]
pub struct DatabaseService {
    client: DatabaseClient,
    worker: Option<JoinHandle<Result<()>>>,
}

/// Shared asynchronous access to the database thread's bounded request queue.
#[derive(Clone, Debug)]
pub struct DatabaseClient {
    requests: mpsc::Sender<Request>,
    closing: Arc<AtomicBool>,
}

impl DatabaseService {
    pub async fn start(cache: SharedCache) -> Result<Self> {
        let (requests, incoming) = mpsc::channel(QUEUE_CAPACITY);
        let (initialized, ready) = oneshot::channel();
        let closing = Arc::new(AtomicBool::new(false));
        let worker_closing = Arc::clone(&closing);
        let worker = thread::Builder::new()
            .name("licht-database".into())
            .spawn(move || {
                let result = run(cache, incoming, initialized, worker_closing);
                if result.is_err() {
                    tracing::error!("Database worker failed");
                }
                result
            })?;
        let service = Self {
            client: DatabaseClient { requests, closing },
            worker: Some(worker),
        };

        match ready.await {
            Ok(Ok(())) => Ok(service),
            Ok(Err(error)) => {
                service.shutdown().await?;
                Err(error)
            }
            Err(_) => {
                service.shutdown().await?;
                Err(unavailable())
            }
        }
    }

    pub fn client(&self) -> DatabaseClient {
        self.client.clone()
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.client.stop();
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };

        // Joining is the only blocking-pool task; SQL always runs on the dedicated thread.
        tokio::task::spawn_blocking(move || {
            worker.join().map_err(|_| CoreError::DatabaseService {
                detail: "worker panicked",
            })?
        })
        .await
        .map_err(|_| CoreError::DatabaseService {
            detail: "could not join the worker",
        })?
    }
}

impl Drop for DatabaseService {
    fn drop(&mut self) {
        // A cancelled startup/shutdown future must still stop and drain the worker.
        self.client.stop();
    }
}

impl DatabaseClient {
    pub async fn last_played_folder(&self) -> Result<Option<String>> {
        let (reply, response) = oneshot::channel();
        self.send(Request::LastPlayed(reply)).await?;
        response.await.map_err(|_| unavailable())?
    }

    pub async fn set_last_played_folder(&self, folder: &str) -> Result<()> {
        let folder = db::validate_last_played_folder(folder)?.to_string();
        let (reply, response) = oneshot::channel();
        self.send(Request::SetLastPlayed(folder, reply)).await?;
        response.await.map_err(|_| unavailable())?
    }

    pub async fn clear_last_played_folder(&self) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(Request::ClearLastPlayed(reply)).await?;
        response.await.map_err(|_| unavailable())?
    }

    async fn send(&self, request: Request) -> Result<()> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(unavailable());
        }
        self.requests.send(request).await.map_err(|_| unavailable())
    }

    fn stop(&self) {
        self.closing.store(true, Ordering::SeqCst);
        match self.requests.try_send(Request::Stop) {
            // Full already guarantees work will wake the worker; closed is already stopped.
            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }
}

fn run(
    cache: SharedCache,
    mut incoming: mpsc::Receiver<Request>,
    initialized: oneshot::Sender<Result<()>>,
    closing: Arc<AtomicBool>,
) -> Result<()> {
    let connection = match db::open_db(&cache) {
        Ok(connection) => connection,
        Err(error) => {
            if let Err(Err(_)) = initialized.send(Err(error)) {
                tracing::error!("Database initialization failed after its caller disconnected");
            }
            return Ok(());
        }
    };
    if initialized.send(Ok(())).is_err() {
        closing.store(true, Ordering::SeqCst);
    }

    loop {
        if closing.load(Ordering::SeqCst) {
            incoming.close();
        }
        let Some(request) = incoming.blocking_recv() else {
            break;
        };

        // Accepted writes execute even when their caller has stopped awaiting the reply.
        match request {
            Request::LastPlayed(reply) => {
                respond(reply, db::get_meta(&connection, META_LAST_PLAYED));
            }
            Request::SetLastPlayed(folder, reply) => {
                respond(reply, db::set_meta(&connection, META_LAST_PLAYED, &folder));
            }
            Request::ClearLastPlayed(reply) => {
                respond(reply, db::clear_meta(&connection, META_LAST_PLAYED));
            }
            Request::Stop => {}
        }
    }

    connection.close().map_err(|(_, error)| error.into())
}

fn respond<T>(reply: oneshot::Sender<Result<T>>, result: Result<T>) {
    if let Err(Err(_)) = reply.send(result) {
        tracing::error!("Database operation failed after its caller disconnected");
    }
}

fn unavailable() -> CoreError {
    CoreError::DatabaseService {
        detail: "worker is closing or unavailable",
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn dropping_the_owner_stops_the_worker_even_when_a_client_survives() {
        let cache = SharedCache::at(
            std::env::temp_dir().join(format!("licht-database-owner-drop-{}", std::process::id())),
        );
        let mut service = DatabaseService::start(cache.clone()).await.expect("start");
        let client = service.client();
        let worker = service.worker.take().expect("worker handle");
        drop(service);

        let joined = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::task::spawn_blocking(move || worker.join()),
        )
        .await;
        let closed = client.last_played_folder().await;
        drop(client);
        joined
            .expect("owner drop must stop the worker without waiting for clients")
            .expect("join task")
            .expect("worker thread")
            .expect("connection close");
        assert!(matches!(closed, Err(CoreError::DatabaseService { .. })));
        std::fs::remove_dir_all(cache.root()).expect("fixture cleanup");
    }
}
