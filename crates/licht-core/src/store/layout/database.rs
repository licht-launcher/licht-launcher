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
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *transaction)
            .await?;
        for mapping in mappings {
            for query in [
                "UPDATE download_jobs SET data_directory = ?1 WHERE data_directory = ?2",
                "UPDATE content_fingerprints SET data_directory = ?1 WHERE data_directory = ?2",
                "UPDATE native_generations SET data_directory = ?1 WHERE data_directory = ?2",
                "UPDATE native_outputs SET data_directory = ?1 WHERE data_directory = ?2",
                "UPDATE instances SET instances_directory = ?1 WHERE instances_directory = ?2",
                "UPDATE modpack_indexes SET instances_directory = ?1 WHERE instances_directory = ?2",
                "UPDATE modpack_dependencies SET instances_directory = ?1 WHERE instances_directory = ?2",
                "UPDATE modpack_files SET instances_directory = ?1 WHERE instances_directory = ?2",
            ] {
                sqlx::query(query)
                    .bind(mapping.destination.to_str().ok_or(CoreError::CachePath)?)
                    .bind(mapping.source.to_str().ok_or(CoreError::CachePath)?)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
        let java_paths: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT instances_directory, instance_folder, java_path FROM instances WHERE java_path IS NOT NULL",
        ).fetch_all(&mut *transaction).await?;
        for (directory, folder, path) in java_paths {
            let relocated = relocate(Path::new(&path), mappings);
            sqlx::query("UPDATE instances SET java_path = ?1 WHERE instances_directory = ?2 AND instance_folder = ?3")
                .bind(relocated.to_str().ok_or(CoreError::CachePath)?)
                .bind(directory).bind(folder).execute(&mut *transaction).await?;
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
