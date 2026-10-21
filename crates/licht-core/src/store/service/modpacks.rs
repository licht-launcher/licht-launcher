use std::path::Path;

use sqlx::SqliteConnection;
use tokio::sync::oneshot;

use super::{DatabaseClient, respond, unavailable};
use crate::modpacks::mrpack::storage::{StoredModpack, StoredModpackFile, StoredModpackHeader};
use crate::store::repositories::modpacks::ModpackRepository;
use crate::{CoreError, Result};

pub(super) enum Request {
    Header(
        String,
        String,
        oneshot::Sender<Result<Option<StoredModpackHeader>>>,
    ),
    Dependencies(
        String,
        String,
        oneshot::Sender<Result<Vec<(String, String)>>>,
    ),
    Files(
        String,
        String,
        oneshot::Sender<Result<Vec<StoredModpackFile>>>,
    ),
    Replace(String, String, StoredModpack, oneshot::Sender<Result<()>>),
    Duplicate(String, String, String, oneshot::Sender<Result<()>>),
    Delete(String, String, oneshot::Sender<Result<()>>),
}

fn directory(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or(CoreError::CachePath)
}

impl DatabaseClient {
    pub(crate) async fn modpack_header(
        &self,
        instances: &Path,
        folder: &str,
    ) -> Result<Option<StoredModpackHeader>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Header(
            directory(instances)?,
            folder.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn modpack_dependencies(
        &self,
        instances: &Path,
        folder: &str,
    ) -> Result<Vec<(String, String)>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Dependencies(
            directory(instances)?,
            folder.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn modpack_files(
        &self,
        instances: &Path,
        folder: &str,
    ) -> Result<Vec<StoredModpackFile>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Files(
            directory(instances)?,
            folder.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn replace_modpack(
        &self,
        instances: &Path,
        folder: &str,
        index: StoredModpack,
    ) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Replace(
            directory(instances)?,
            folder.to_string(),
            index,
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn duplicate_modpack(
        &self,
        instances: &Path,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Duplicate(
            directory(instances)?,
            source.to_string(),
            destination.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn delete_modpack(&self, instances: &Path, folder: &str) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Modpacks(Request::Delete(
            directory(instances)?,
            folder.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }
}

pub(super) async fn execute(connection: &mut SqliteConnection, request: Request) {
    match request {
        Request::Header(directory, folder, reply) => respond(
            reply,
            ModpackRepository::header(connection, &directory, &folder).await,
        ),
        Request::Dependencies(directory, folder, reply) => respond(
            reply,
            ModpackRepository::dependencies(connection, &directory, &folder).await,
        ),
        Request::Files(directory, folder, reply) => respond(
            reply,
            ModpackRepository::files(connection, &directory, &folder).await,
        ),
        Request::Replace(directory, folder, index, reply) => respond(
            reply,
            ModpackRepository::replace(connection, &directory, &folder, &index).await,
        ),
        Request::Duplicate(directory, source, destination, reply) => respond(
            reply,
            ModpackRepository::duplicate(connection, &directory, &source, &destination).await,
        ),
        Request::Delete(directory, folder, reply) => {
            use sqlx::Connection;
            let result = async {
                let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
                ModpackRepository::delete(&mut transaction, &directory, &folder).await?;
                transaction.commit().await?;
                Ok(())
            }
            .await;
            respond(reply, result);
        }
    }
}
