use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{OptionalExtension, TransactionBehavior};

use crate::{CoreError, Result, SharedCache};

const DB_FILE: &str = "licht.sqlite";
const SCHEMA_VERSION: u32 = 4;
const META_LAST_PLAYED: &str = "last_played_folder";

pub fn db_path(cache: &SharedCache) -> PathBuf {
    cache.root().join(DB_FILE)
}

pub fn open_db(cache: &SharedCache) -> Result<rusqlite::Connection> {
    std::fs::create_dir_all(cache.root())?;
    let mut connection = rusqlite::Connection::open(db_path(cache))?;
    connection.busy_timeout(Duration::from_secs(5))?;
    migrate(&mut connection)?;
    Ok(connection)
}

fn migrate(connection: &mut rusqlite::Connection) -> Result<()> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(CoreError::DatabaseSchema { schema: version });
    }
    if version == SCHEMA_VERSION {
        return Ok(());
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Another connection may have migrated while we waited for the writer lock.
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(CoreError::DatabaseSchema { schema: version });
    }
    if version < 1 {
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );",
        )?;
    }
    if version < 2 {
        transaction.execute_batch(
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
        )?;
    }
    if version < 3 {
        transaction.execute_batch(
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
        )?;
    }
    if version < 4 {
        transaction.execute_batch(
            "CREATE INDEX download_job_artifacts_path ON download_job_artifacts (path, job_id);",
        )?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn get_meta(connection: &rusqlite::Connection, key: &str) -> Result<Option<String>> {
    let value = connection
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(value)
}

pub fn set_meta(connection: &rusqlite::Connection, key: &str, value: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

pub fn clear_meta(connection: &rusqlite::Connection, key: &str) -> Result<()> {
    connection.execute("DELETE FROM meta WHERE key = ?1", [key])?;
    Ok(())
}

pub fn last_played_folder(cache: &SharedCache) -> Result<Option<String>> {
    let connection = open_db(cache)?;
    get_meta(&connection, META_LAST_PLAYED)
}

pub fn set_last_played_folder(cache: &SharedCache, folder: &str) -> Result<()> {
    let folder = folder.trim();
    if folder.is_empty() {
        return Err(CoreError::InstanceName);
    }
    let connection = open_db(cache)?;
    set_meta(&connection, META_LAST_PLAYED, folder)
}

pub fn clear_last_played_folder(cache: &SharedCache) -> Result<()> {
    let connection = open_db(cache)?;
    clear_meta(&connection, META_LAST_PLAYED)
}
