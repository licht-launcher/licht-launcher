pub(crate) mod model;

pub use model::{
    DownloadComponent, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus,
    DownloadJobUpdate, DownloadPhase,
};

use crate::store::repositories::download_jobs::DownloadJobRepository;
use crate::{CoreError, Database, Result, SharedCache};

pub async fn create_download_job(
    settings: &Database,
    game: &SharedCache,
    folder: &str,
    version: &str,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    create_pending_download_job(settings, game, folder, Some(version), operation).await
}

pub async fn create_pending_download_job(
    settings: &Database,
    game: &SharedCache,
    folder: &str,
    version: Option<&str>,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    let mut connection = settings.acquire().await?;
    DownloadJobRepository::create(&mut connection, game, folder, version, operation).await
}

pub async fn list_download_jobs(
    settings: &Database,
    game: &SharedCache,
) -> Result<Vec<DownloadJobSnapshot>> {
    let mut connection = settings.acquire().await?;
    DownloadJobRepository::list(&mut connection, game).await
}

pub async fn get_download_job(
    settings: &Database,
    game: &SharedCache,
    id: &str,
) -> Result<DownloadJobSnapshot> {
    let mut connection = settings.acquire().await?;
    DownloadJobRepository::get(&mut connection, game, id).await
}

pub async fn update_download_job(
    settings: &Database,
    game: &SharedCache,
    id: &str,
    update: DownloadJobUpdate,
) -> Result<DownloadJobSnapshot> {
    let mut connection = settings.acquire().await?;
    DownloadJobRepository::update(&mut connection, game, id, update).await
}

pub async fn recover_download_jobs(settings: &Database, game: &SharedCache) -> Result<usize> {
    let mut connection = settings.acquire().await?;
    DownloadJobRepository::recover(&mut connection, game).await
}

fn invalid(detail: &'static str) -> CoreError {
    CoreError::DownloadJobInvalid { detail }
}
