use std::path::PathBuf;
use std::time::Duration;

use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

use super::repositories::metadata::MetadataRepository;

use crate::{CoreError, Result, SharedCache};

const DB_FILE: &str = "licht.sqlite";
const SCHEMA_VERSION: u32 = 4;
pub(super) const META_LAST_PLAYED: &str = "last_played_folder";

pub fn db_path(cache: &SharedCache) -> PathBuf {
    cache.root().join(DB_FILE)
}

pub async fn open_db(cache: &SharedCache) -> Result<SqliteConnection> {
    tokio::fs::create_dir_all(cache.root()).await?;
    let options = SqliteConnectOptions::new()
        .filename(db_path(cache))
        .create_if_missing(true)
        .busy_timeout(Duration::from_secs(5));
    let mut connection = SqliteConnection::connect_with(&options).await?;
    let migrated = migrate(&mut connection).await;
    if let Err(error) = migrated {
        if connection.close().await.is_err() {
            tracing::error!("Failed to close the database after a migration error");
        }
        return Err(error);
    }
    Ok(connection)
}

pub(crate) async fn complete_operation<T>(
    connection: SqliteConnection,
    result: Result<T>,
) -> Result<T> {
    match (result, connection.close().await) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(error), closed) => {
            if closed.is_err() {
                tracing::error!("Failed to close the database after an operation error");
            }
            Err(error)
        }
    }
}

async fn schema_version(connection: &mut SqliteConnection) -> Result<u32> {
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(connection)
        .await?;
    u32::try_from(version).map_err(|error| sqlx::Error::Decode(Box::new(error)).into())
}

async fn migrate(connection: &mut SqliteConnection) -> Result<()> {
    let version = schema_version(connection).await?;
    if version > SCHEMA_VERSION {
        return Err(CoreError::DatabaseSchema { schema: version });
    }
    if version == SCHEMA_VERSION {
        return Ok(());
    }

    let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
    // Another connection may have migrated while we waited for the writer lock.
    let version = schema_version(&mut transaction).await?;
    if version > SCHEMA_VERSION {
        return Err(CoreError::DatabaseSchema { schema: version });
    }
    if version < 1 {
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );",
        )
        .execute(&mut *transaction)
        .await?;
    }
    if version < 2 {
        sqlx::raw_sql(
            "CREATE TABLE download_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data_directory TEXT NOT NULL,
                instance_folder TEXT NOT NULL,
                version_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                status TEXT NOT NULL,
                component TEXT,
                phase TEXT NOT NULL,
                progress TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX download_jobs_directory ON download_jobs (data_directory, id);",
        )
        .execute(&mut *transaction)
        .await?;
    }
    if version < 3 {
        sqlx::raw_sql(
            "CREATE TABLE download_jobs_next (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data_directory TEXT NOT NULL, instance_folder TEXT,
                version_id TEXT, operation TEXT NOT NULL, status TEXT NOT NULL,
                component TEXT, phase TEXT NOT NULL, progress TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, error TEXT, java_major INTEGER
            );
            INSERT INTO download_jobs_next
                SELECT id, data_directory, instance_folder, version_id, operation, status,
                       component, phase, progress, created_at, updated_at, NULL, NULL FROM download_jobs;
            DROP TABLE download_jobs;
            ALTER TABLE download_jobs_next RENAME TO download_jobs;
            CREATE INDEX download_jobs_directory ON download_jobs (data_directory, id);
            CREATE TABLE download_job_artifacts (
                job_id INTEGER NOT NULL, path TEXT NOT NULL,
                PRIMARY KEY (job_id, path)
            );",
        ).execute(&mut *transaction).await?;
    }
    if version < 4 {
        sqlx::raw_sql(
            "CREATE INDEX download_job_artifacts_path ON download_job_artifacts (path, job_id);",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query("PRAGMA user_version = 4")
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn get_meta(connection: &mut SqliteConnection, key: &str) -> Result<Option<String>> {
    MetadataRepository::get(connection, key).await
}

pub async fn set_meta(connection: &mut SqliteConnection, key: &str, value: &str) -> Result<()> {
    MetadataRepository::set(connection, key, value).await
}

pub async fn clear_meta(connection: &mut SqliteConnection, key: &str) -> Result<()> {
    MetadataRepository::clear(connection, key).await
}

pub async fn last_played_folder(cache: &SharedCache) -> Result<Option<String>> {
    let mut connection = open_db(cache).await?;
    let result = get_meta(&mut connection, META_LAST_PLAYED).await;
    complete_operation(connection, result).await
}

pub async fn set_last_played_folder(cache: &SharedCache, folder: &str) -> Result<()> {
    let folder = validate_last_played_folder(folder)?;
    let mut connection = open_db(cache).await?;
    let result = set_meta(&mut connection, META_LAST_PLAYED, folder).await;
    complete_operation(connection, result).await
}

pub(super) fn validate_last_played_folder(folder: &str) -> Result<&str> {
    let folder = folder.trim();
    if folder.is_empty() {
        return Err(CoreError::InstanceName);
    }
    Ok(folder)
}

pub async fn clear_last_played_folder(cache: &SharedCache) -> Result<()> {
    let mut connection = open_db(cache).await?;
    let result = clear_meta(&mut connection, META_LAST_PLAYED).await;
    complete_operation(connection, result).await
}
