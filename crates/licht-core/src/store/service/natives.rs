use sqlx::SqliteConnection;
use tokio::sync::oneshot;

use super::{DatabaseClient, respond, unavailable};
use crate::store::repositories::natives::{NativeGeneration, NativeRepository};
use crate::{Result, SharedCache};

type Reply<T> = oneshot::Sender<Result<T>>;

pub(super) enum Request {
    Get(SharedCache, String, String, Reply<Option<NativeGeneration>>),
    Put(SharedCache, String, String, NativeGeneration, Reply<()>),
    Invalidate(SharedCache, String, Reply<()>),
}

impl DatabaseClient {
    pub(crate) async fn native_generation(
        &self,
        game: &SharedCache,
        version: &str,
        fingerprint: &str,
    ) -> Result<Option<NativeGeneration>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Natives(Request::Get(
            game.clone(),
            version.to_string(),
            fingerprint.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn store_native_generation(
        &self,
        game: &SharedCache,
        version: &str,
        fingerprint: &str,
        generation: NativeGeneration,
    ) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Natives(Request::Put(
            game.clone(),
            version.to_string(),
            fingerprint.to_string(),
            generation,
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn invalidate_native_generations(
        &self,
        game: &SharedCache,
        version: &str,
    ) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Natives(Request::Invalidate(
            game.clone(),
            version.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }
}

pub(super) async fn execute(connection: &mut SqliteConnection, request: Request) {
    match request {
        Request::Get(game, version, fingerprint, reply) => respond(
            reply,
            NativeRepository::get(connection, &game, &version, &fingerprint).await,
        ),
        Request::Put(game, version, fingerprint, generation, reply) => respond(
            reply,
            NativeRepository::put(connection, &game, &version, &fingerprint, &generation).await,
        ),
        Request::Invalidate(game, version, reply) => respond(
            reply,
            NativeRepository::invalidate(connection, &game, &version).await,
        ),
    }
}
