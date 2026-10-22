use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sqlx::Connection;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use super::db::{self, META_LAST_PLAYED};
use crate::{CoreError, Result, SharedCache};

const QUEUE_CAPACITY: usize = 64;

mod content;
mod downloads;
mod instances;
mod modpacks;
mod natives;

enum Request {
    LastPlayed(oneshot::Sender<Result<Option<String>>>),
    SetLastPlayed(String, oneshot::Sender<Result<()>>),
    ClearLastPlayed(oneshot::Sender<Result<()>>),
    Downloads(downloads::Request),
    Content(content::Request),
    Modpacks(modpacks::Request),
    Instances(instances::Request),
    Natives(natives::Request),
    RememberAccount(String, bool, oneshot::Sender<Result<()>>),
    AccountUuids(oneshot::Sender<Result<Vec<String>>>),
    Stop,
}

/// Owns the dispatcher and persistent SQLx connection. Shutdown waits for accepted work and closure.
#[derive(Debug)]
pub struct DatabaseService {
    client: DatabaseClient,
    worker: Option<JoinHandle<Result<()>>>,
}

/// Shared asynchronous access to the database dispatcher's bounded request queue.
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
        let worker = tokio::spawn(async move {
            let result = run(cache, incoming, initialized, worker_closing).await;
            if result.is_err() {
                tracing::error!("Database worker failed");
            }
            result
        });
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

        worker.await.map_err(|_| CoreError::DatabaseService {
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
    pub async fn remembered_account_uuids(&self) -> Result<Vec<String>> {
        let (reply, response) = oneshot::channel();
        self.send(Request::AccountUuids(reply)).await?;
        response.await.map_err(|_| unavailable())?
    }

    pub async fn remember_account_uuid(&self, uuid: &str, remember: bool) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(Request::RememberAccount(uuid.to_string(), remember, reply))
            .await?;
        response.await.map_err(|_| unavailable())?
    }

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

async fn run(
    cache: SharedCache,
    mut incoming: mpsc::Receiver<Request>,
    initialized: oneshot::Sender<Result<()>>,
    closing: Arc<AtomicBool>,
) -> Result<()> {
    let mut connection = match db::open_db(&cache).await {
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
        let Some(request) = incoming.recv().await else {
            break;
        };

        // Accepted writes execute even when their caller has stopped awaiting the reply.
        match request {
            Request::LastPlayed(reply) => {
                respond(reply, db::get_meta(&mut connection, META_LAST_PLAYED).await);
            }
            Request::SetLastPlayed(folder, reply) => {
                respond(
                    reply,
                    db::set_meta(&mut connection, META_LAST_PLAYED, &folder).await,
                );
            }
            Request::ClearLastPlayed(reply) => {
                respond(
                    reply,
                    db::clear_meta(&mut connection, META_LAST_PLAYED).await,
                );
            }
            Request::Stop => {}
            Request::Downloads(request) => downloads::execute(&mut connection, request).await,
            Request::Content(request) => content::execute(&mut connection, request).await,
            Request::Modpacks(request) => modpacks::execute(&mut connection, request).await,
            Request::Instances(request) => instances::execute(&mut connection, request).await,
            Request::Natives(request) => natives::execute(&mut connection, request).await,
            Request::AccountUuids(reply) => {
                let result = db::get_meta(&mut connection, "microsoft_account_uuids")
                    .await
                    .map(|raw| {
                        raw.unwrap_or_default()
                            .split(',')
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                            .collect()
                    });
                respond(reply, result);
            }
            Request::RememberAccount(uuid, remember, reply) => {
                respond(
                    reply,
                    super::repositories::metadata::MetadataRepository::update_account(
                        &mut connection,
                        &uuid,
                        remember,
                    )
                    .await,
                );
            }
        }
    }

    connection.close().await.map_err(Into::into)
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

        let joined = tokio::time::timeout(Duration::from_secs(10), worker).await;
        let closed = client.last_played_folder().await;
        drop(client);
        joined
            .expect("owner drop must stop the worker without waiting for clients")
            .expect("join task")
            .expect("connection close");
        assert!(matches!(closed, Err(CoreError::DatabaseService { .. })));
        std::fs::remove_dir_all(cache.root()).expect("fixture cleanup");
    }
}
