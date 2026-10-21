use std::path::Path;

use sqlx::SqliteConnection;
use tokio::sync::oneshot;

use super::{DatabaseClient, respond, unavailable};
use crate::store::repositories::instances::InstanceRepository;
use crate::{CoreError, Instance, Result};

pub(super) enum Request {
    Get(String, String, oneshot::Sender<Result<Instance>>),
    List(String, oneshot::Sender<Result<Vec<(String, Instance)>>>),
    Write(String, String, Box<Instance>, oneshot::Sender<Result<()>>),
    Delete(String, String, oneshot::Sender<Result<()>>),
}

fn directory(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or(CoreError::CachePath)
}

fn key(path: &Path) -> Result<(String, String)> {
    let parent = path.parent().ok_or(CoreError::CachePath)?;
    let folder = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CoreError::CachePath)?;
    crate::instance::single_component(folder)?;
    Ok((directory(parent)?, folder.to_string()))
}

impl DatabaseClient {
    pub(crate) async fn instance_profile(&self, path: &Path) -> Result<Instance> {
        let (directory, folder) = key(path)?;
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Instances(Request::Get(
            directory, folder, reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn instance_profiles(&self, path: &Path) -> Result<Vec<(String, Instance)>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Instances(Request::List(
            directory(path)?,
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn write_instance_profile(
        &self,
        path: &Path,
        instance: &Instance,
    ) -> Result<()> {
        let (directory, folder) = key(path)?;
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Instances(Request::Write(
            directory,
            folder,
            Box::new(instance.clone()),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn delete_instance_profile(&self, path: &Path) -> Result<()> {
        let (directory, folder) = key(path)?;
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Instances(Request::Delete(
            directory, folder, reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }
}

pub(super) async fn execute(connection: &mut SqliteConnection, request: Request) {
    match request {
        Request::Get(directory, folder, reply) => respond(
            reply,
            InstanceRepository::get(connection, &directory, &folder).await,
        ),
        Request::List(directory, reply) => respond(
            reply,
            InstanceRepository::list(connection, &directory).await,
        ),
        Request::Write(directory, folder, instance, reply) => respond(
            reply,
            InstanceRepository::write(connection, &directory, &folder, &instance).await,
        ),
        Request::Delete(directory, folder, reply) => respond(
            reply,
            InstanceRepository::delete(connection, &directory, &folder).await,
        ),
    }
}
