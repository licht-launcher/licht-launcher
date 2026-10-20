use std::path::PathBuf;

use sqlx::SqliteConnection;
use tokio::sync::oneshot;

use super::{DatabaseClient, respond, unavailable};
use crate::store::repositories::{
    download_artifacts::DownloadArtifactRepository, download_jobs::DownloadJobRepository,
};
use crate::{DownloadJobOperation, DownloadJobSnapshot, DownloadJobUpdate, Result, SharedCache};

type Reply<T> = oneshot::Sender<Result<T>>;

pub(super) enum Request {
    Create {
        game: SharedCache,
        folder: String,
        version: Option<String>,
        operation: DownloadJobOperation,
        reply: Reply<DownloadJobSnapshot>,
    },
    List(SharedCache, Reply<Vec<DownloadJobSnapshot>>),
    Get(SharedCache, String, Reply<DownloadJobSnapshot>),
    Update(
        SharedCache,
        String,
        DownloadJobUpdate,
        Reply<DownloadJobSnapshot>,
    ),
    Recover(SharedCache, Reply<usize>),
    Version(SharedCache, String, String, Reply<()>),
    Java(SharedCache, u32, Reply<DownloadJobSnapshot>),
    Register(SharedCache, String, Vec<PathBuf>, Reply<()>),
    Cleanup(SharedCache, String, Reply<Vec<PathBuf>>),
}

impl DatabaseClient {
    pub async fn create_pending_download_job(
        &self,
        game: &SharedCache,
        folder: &str,
        version: Option<&str>,
        operation: DownloadJobOperation,
    ) -> Result<DownloadJobSnapshot> {
        self.download_request(|reply| Request::Create {
            game: game.clone(),
            folder: folder.to_string(),
            version: version.map(str::to_string),
            operation,
            reply,
        })
        .await
    }

    pub async fn list_download_jobs(&self, game: &SharedCache) -> Result<Vec<DownloadJobSnapshot>> {
        self.download_request(|reply| Request::List(game.clone(), reply))
            .await
    }

    pub async fn get_download_job(
        &self,
        game: &SharedCache,
        id: &str,
    ) -> Result<DownloadJobSnapshot> {
        self.download_request(|reply| Request::Get(game.clone(), id.to_string(), reply))
            .await
    }

    pub async fn update_download_job(
        &self,
        game: &SharedCache,
        id: &str,
        update: DownloadJobUpdate,
    ) -> Result<DownloadJobSnapshot> {
        self.download_request(|reply| Request::Update(game.clone(), id.to_string(), update, reply))
            .await
    }

    pub async fn recover_download_jobs(&self, game: &SharedCache) -> Result<usize> {
        self.download_request(|reply| Request::Recover(game.clone(), reply))
            .await
    }

    pub async fn set_download_job_version(
        &self,
        game: &SharedCache,
        id: &str,
        version: &str,
    ) -> Result<()> {
        self.download_request(|reply| {
            Request::Version(game.clone(), id.to_string(), version.to_string(), reply)
        })
        .await
    }

    pub async fn create_java_download_job(
        &self,
        game: &SharedCache,
        major: u32,
    ) -> Result<DownloadJobSnapshot> {
        self.download_request(|reply| Request::Java(game.clone(), major, reply))
            .await
    }

    pub async fn register_download_artifacts(
        &self,
        game: &SharedCache,
        id: &str,
        paths: &[PathBuf],
    ) -> Result<()> {
        self.download_request(|reply| {
            Request::Register(game.clone(), id.to_string(), paths.to_vec(), reply)
        })
        .await
    }

    /// Returns validated candidates; destination locking and file removal happen outside SQLite.
    pub async fn download_cleanup_candidates(
        &self,
        game: &SharedCache,
        id: &str,
    ) -> Result<Vec<PathBuf>> {
        self.download_request(|reply| Request::Cleanup(game.clone(), id.to_string(), reply))
            .await
    }

    async fn download_request<T>(&self, request: impl FnOnce(Reply<T>) -> Request) -> Result<T> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Downloads(request(reply))).await?;
        response.await.map_err(|_| unavailable())?
    }
}

pub(super) async fn execute(connection: &mut SqliteConnection, request: Request) {
    match request {
        Request::Create {
            game,
            folder,
            version,
            operation,
            reply,
        } => {
            respond(
                reply,
                DownloadJobRepository::create(
                    connection,
                    &game,
                    &folder,
                    version.as_deref(),
                    operation,
                )
                .await,
            );
        }
        Request::List(game, reply) => {
            respond(reply, DownloadJobRepository::list(connection, &game).await)
        }
        Request::Get(game, id, reply) => respond(
            reply,
            DownloadJobRepository::get(connection, &game, &id).await,
        ),
        Request::Update(game, id, update, reply) => respond(
            reply,
            DownloadJobRepository::update(connection, &game, &id, update).await,
        ),
        Request::Recover(game, reply) => respond(
            reply,
            DownloadJobRepository::recover(connection, &game).await,
        ),
        Request::Version(game, id, version, reply) => respond(
            reply,
            DownloadJobRepository::set_version(connection, &game, &id, &version).await,
        ),
        Request::Java(game, major, reply) => respond(
            reply,
            DownloadJobRepository::create_java(connection, &game, major).await,
        ),
        Request::Register(game, id, paths, reply) => respond(
            reply,
            DownloadArtifactRepository::register(connection, &game, &id, &paths).await,
        ),
        Request::Cleanup(game, id, reply) => respond(
            reply,
            DownloadArtifactRepository::cleanup_candidates(connection, &game, &id).await,
        ),
    }
}
