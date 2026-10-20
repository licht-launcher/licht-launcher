use std::path::{Component, Path, PathBuf};

use sqlx::{Connection, SqliteConnection};

use super::download_jobs::{DownloadJobRepository, encode, parse_id};
use crate::{CoreError, DownloadJobStatus, Result, SharedCache};

pub(crate) struct DownloadArtifactRepository;

impl DownloadArtifactRepository {
    pub async fn register(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
        paths: &[PathBuf],
    ) -> Result<()> {
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let job = DownloadJobRepository::get(&mut transaction, game, id).await?;
        let instance = job
            .instance_folder
            .as_deref()
            .map(|folder| game.instance_dir(folder))
            .transpose()?;
        for path in paths {
            validate_artifact(game, instance.as_deref(), path)?;
            sqlx::query(
                "INSERT OR IGNORE INTO download_job_artifacts (job_id, path) VALUES (?1, ?2)",
            )
            .bind(parse_id(id)?)
            .bind(path.to_str().ok_or(CoreError::CachePath)?)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn cleanup_candidates(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
    ) -> Result<Vec<PathBuf>> {
        let job = DownloadJobRepository::get(connection, game, id).await?;
        let paths: Vec<String> = sqlx::query_scalar(
            "SELECT artifact.path FROM download_job_artifacts artifact WHERE artifact.job_id = ?1
             AND NOT EXISTS (SELECT 1 FROM download_job_artifacts other
                 JOIN download_jobs job ON job.id = other.job_id
                 WHERE other.path = artifact.path AND other.job_id != ?1
                 AND job.status NOT IN (?2, ?3))",
        )
        .bind(parse_id(id)?)
        .bind(encode(&DownloadJobStatus::Completed)?)
        .bind(encode(&DownloadJobStatus::Cancelled)?)
        .fetch_all(connection)
        .await?;
        let instance = job
            .instance_folder
            .as_deref()
            .map(|folder| game.instance_dir(folder))
            .transpose()?;
        paths
            .into_iter()
            .map(|path| {
                let path = PathBuf::from(path);
                validate_artifact(game, instance.as_deref(), &path)?;
                Ok(path)
            })
            .collect()
    }
}

fn validate_artifact(game: &SharedCache, instance: Option<&Path>, path: &Path) -> Result<()> {
    if !path.is_absolute()
        || (!path.starts_with(game.root())
            && !instance.is_some_and(|instance| path.starts_with(instance)))
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(CoreError::CachePath);
    }
    Ok(())
}
