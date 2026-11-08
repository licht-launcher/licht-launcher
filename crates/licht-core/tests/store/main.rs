//! Integration tests for cache, database, and settings (`store/`).

mod cache;
mod db;
mod download_jobs;
mod service;
mod settings;

/// Migrates the profile database as startup does, then returns a plain connection apart
/// from any pool, for fixtures that hold locks or change the schema.
async fn open_migrated(
    cache: &licht_core::SharedCache,
) -> licht_core::Result<sqlx::SqliteConnection> {
    use sqlx::Connection;
    licht_core::Database::open(cache).await?.close().await;
    Ok(sqlx::SqliteConnection::connect_with(
        &sqlx::sqlite::SqliteConnectOptions::new()
            .filename(licht_core::db_path(cache))
            .busy_timeout(std::time::Duration::from_secs(5)),
    )
    .await?)
}

/// Reads the last played folder as a separate caller would, through its own short-lived pool.
async fn stored_last_played(cache: &licht_core::SharedCache) -> licht_core::Result<Option<String>> {
    let database = licht_core::Database::open(cache).await?;
    let folder = licht_core::last_played_folder(&database).await;
    database.close().await;
    folder
}
