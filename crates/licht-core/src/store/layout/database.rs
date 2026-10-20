use std::path::Path;
use std::time::Duration;

use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

use super::{Relocation, relocate};
use crate::{CoreError, Result, SharedCache};

pub(super) async fn snapshot(legacy: &Path, staging: &SharedCache) -> Result<()> {
    if legacy.exists() {
        let mut source = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(legacy)
                .read_only(true)
                .busy_timeout(Duration::from_secs(5)),
        )
        .await?;
        // SQLite reads a consistent snapshot, including committed WAL frames.
        let result = sqlx::query("VACUUM INTO ?1")
            .bind(
                super::db::db_path(staging)
                    .to_str()
                    .ok_or(CoreError::CachePath)?,
            )
            .execute(&mut source)
            .await;
        source.close().await?;
        result?;
    }
    Ok(())
}

pub(super) async fn remap(staging: &SharedCache, mappings: &[Relocation]) -> Result<()> {
    let mut connection = super::db::open_db(staging).await?;
    let result = async {
        let mut transaction = connection.begin().await?;
        for mapping in mappings {
            for query in [
                "UPDATE download_jobs SET data_directory = ?1 WHERE data_directory = ?2",
                "UPDATE content_fingerprints SET data_directory = ?1 WHERE data_directory = ?2",
            ] {
                sqlx::query(query)
                    .bind(mapping.destination.to_str().ok_or(CoreError::CachePath)?)
                    .bind(mapping.source.to_str().ok_or(CoreError::CachePath)?)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
        let artifacts: Vec<(i64, String)> =
            sqlx::query_as("SELECT job_id, path FROM download_job_artifacts")
                .fetch_all(&mut *transaction)
                .await?;
        sqlx::query("DELETE FROM download_job_artifacts")
            .execute(&mut *transaction)
            .await?;
        for (id, path) in artifacts {
            let path = relocate(Path::new(&path), mappings);
            sqlx::query(
                "INSERT OR IGNORE INTO download_job_artifacts (job_id, path) VALUES (?1, ?2)",
            )
            .bind(id)
            .bind(path.to_str().ok_or(CoreError::CachePath)?)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&mut connection)
            .await?;
        if result != "ok" {
            return Err(
                std::io::Error::other("migrated database failed integrity validation").into(),
            );
        }
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&mut connection)
            .await?;
        Ok(())
    }
    .await;
    super::db::complete_operation(connection, result).await
}
