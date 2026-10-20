pub(crate) mod model;

pub use model::{
    DownloadComponent, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus,
    DownloadJobUpdate, DownloadPhase,
};

use crate::store::repositories::download_jobs::DownloadJobRepository;
use crate::{CoreError, Result, SharedCache, open_db};

pub async fn create_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    folder: &str,
    version: &str,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    create_pending_download_job(settings, game, folder, Some(version), operation).await
}

pub async fn create_pending_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    folder: &str,
    version: Option<&str>,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    let mut connection = open_db(settings).await?;
    let result =
        DownloadJobRepository::create(&mut connection, game, folder, version, operation).await;
    crate::db::complete_operation(connection, result).await
}

pub async fn list_download_jobs(
    settings: &SharedCache,
    game: &SharedCache,
) -> Result<Vec<DownloadJobSnapshot>> {
    let mut connection = open_db(settings).await?;
    let result = DownloadJobRepository::list(&mut connection, game).await;
    crate::db::complete_operation(connection, result).await
}

pub async fn get_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
) -> Result<DownloadJobSnapshot> {
    let mut connection = open_db(settings).await?;
    let result = DownloadJobRepository::get(&mut connection, game, id).await;
    crate::db::complete_operation(connection, result).await
}

pub async fn update_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
    update: DownloadJobUpdate,
) -> Result<DownloadJobSnapshot> {
    let mut connection = open_db(settings).await?;
    let result = DownloadJobRepository::update(&mut connection, game, id, update).await;
    crate::db::complete_operation(connection, result).await
}

pub async fn recover_download_jobs(settings: &SharedCache, game: &SharedCache) -> Result<usize> {
    let mut connection = open_db(settings).await?;
    let result = DownloadJobRepository::recover(&mut connection, game).await;
    crate::db::complete_operation(connection, result).await
}

fn invalid(detail: &'static str) -> CoreError {
    CoreError::DownloadJobInvalid { detail }
}
