use sqlx::{Connection, SqliteConnection};

use super::download_jobs::{DownloadJobRepository, parse_id};
use crate::transfer::jobs::model::DownloadCheckpoint;
use crate::{CoreError, DownloadJobStatus, Result, SharedCache};

pub(crate) struct DownloadCheckpointRepository;

impl DownloadCheckpointRepository {
    pub async fn contains(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
        version: &str,
        stage: DownloadCheckpoint,
    ) -> Result<bool> {
        game.version_json(version)?;
        DownloadJobRepository::get(connection, game, id).await?;
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM download_job_checkpoints WHERE job_id = ?1 AND stage = ?2 AND version_id = ?3)")
            .bind(parse_id(id)?).bind(stage.as_str()).bind(version).fetch_one(connection).await?)
    }

    pub async fn record(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
        version: &str,
        stage: DownloadCheckpoint,
    ) -> Result<()> {
        game.version_json(version)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let job = DownloadJobRepository::get(&mut transaction, game, id).await?;
        if matches!(
            job.status,
            DownloadJobStatus::Cancelled
                | DownloadJobStatus::Cancelling
                | DownloadJobStatus::Completed
        ) {
            return Err(CoreError::DownloadCancelled);
        }
        sqlx::query(
            "INSERT INTO download_job_checkpoints (job_id, stage, version_id) VALUES (?1, ?2, ?3)
            ON CONFLICT(job_id, stage) DO UPDATE SET version_id = excluded.version_id",
        )
        .bind(parse_id(id)?)
        .bind(stage.as_str())
        .bind(version)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabaseService, DownloadJobOperation};

    #[tokio::test]
    async fn checkpoints_survive_reopening_and_are_scoped_to_job_directory_and_version() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("licht-job-checkpoints-{stamp}"));
        let home = SharedCache::at(&root);
        let game = SharedCache::at(root.join("data"));
        let service = DatabaseService::start(home.clone()).await.expect("service");
        let client = service.client();
        let first = client
            .create_pending_download_job(
                &game,
                "First",
                Some("1.21"),
                DownloadJobOperation::Install,
            )
            .await
            .expect("job");
        let second = client
            .create_pending_download_job(
                &game,
                "Second",
                Some("1.21"),
                DownloadJobOperation::Repair,
            )
            .await
            .expect("repair");
        client
            .record_download_checkpoint(&game, &first.id, "1.21", DownloadCheckpoint::Game)
            .await
            .expect("checkpoint");
        assert!(
            !client
                .has_download_checkpoint(&game, &second.id, "1.21", DownloadCheckpoint::Game)
                .await
                .expect("new repair")
        );
        assert!(
            !client
                .has_download_checkpoint(&game, &first.id, "1.20", DownloadCheckpoint::Game)
                .await
                .expect("other version")
        );
        let other = SharedCache::at(root.join("other-data"));
        assert!(matches!(
            client
                .has_download_checkpoint(&other, &first.id, "1.21", DownloadCheckpoint::Game)
                .await,
            Err(CoreError::DownloadJobMissing)
        ));
        service.shutdown().await.expect("shutdown");
        let service = DatabaseService::start(home).await.expect("reopen");
        let client = service.client();
        assert!(
            client
                .has_download_checkpoint(&game, &first.id, "1.21", DownloadCheckpoint::Game)
                .await
                .expect("persisted checkpoint")
        );
        assert!(
            !client
                .has_download_checkpoint(&game, &first.id, "1.21", DownloadCheckpoint::Modpack)
                .await
                .expect("unfinished stage")
        );
        client
            .set_download_job_version(&game, &first.id, "1.20")
            .await
            .expect("changed version");
        assert!(
            !client
                .has_download_checkpoint(&game, &first.id, "1.21", DownloadCheckpoint::Game)
                .await
                .expect("invalidated checkpoint")
        );
        service.shutdown().await.expect("shutdown");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
