use licht_core::{
    CoreError, SharedCache, clear_last_played_folder, db_path, last_played_folder, open_db,
    set_last_played_folder,
};

fn cache(name: &str) -> SharedCache {
    let root = std::env::temp_dir().join(format!("licht-db-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    SharedCache::at(root)
}

#[test]
fn open_creates_the_database_and_is_idempotent() {
    let cache = cache("open");
    let first = open_db(&cache).expect("open");
    drop(first);
    assert!(db_path(&cache).is_file());
    open_db(&cache).expect("reopen");
}

#[test]
fn last_played_folder_round_trips() {
    let cache = cache("last-played");
    assert_eq!(last_played_folder(&cache).expect("empty"), None);

    set_last_played_folder(&cache, "classic").expect("set");
    assert_eq!(
        last_played_folder(&cache).expect("read").as_deref(),
        Some("classic")
    );

    set_last_played_folder(&cache, "survival").expect("update");
    assert_eq!(
        last_played_folder(&cache).expect("read").as_deref(),
        Some("survival")
    );

    clear_last_played_folder(&cache).expect("clear");
    assert_eq!(last_played_folder(&cache).expect("cleared"), None);
}

#[test]
fn a_version_one_database_gains_download_jobs_without_losing_existing_metadata() {
    let cache = cache("downloads-migration");
    std::fs::create_dir_all(cache.root()).expect("cache directory");
    let connection = rusqlite::Connection::open(db_path(&cache)).expect("legacy database");
    connection
        .execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
             INSERT INTO meta (key, value) VALUES ('last_played_folder', 'survival');
             PRAGMA user_version = 1;",
        )
        .expect("legacy schema");
    drop(connection);

    let connection = open_db(&cache).expect("migrate");
    let tables: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'download_jobs'",
            [],
            |row| row.get(0),
        )
        .expect("download table");
    assert_eq!(tables, 1);
    assert_eq!(
        last_played_folder(&cache).expect("metadata"),
        Some("survival".to_string())
    );
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version");
    assert_eq!(version, 2);
}

#[test]
fn a_newer_database_schema_is_rejected_without_downgrading_it() {
    let cache = cache("future-schema");
    std::fs::create_dir_all(cache.root()).expect("cache directory");
    let connection = rusqlite::Connection::open(db_path(&cache)).expect("database");
    connection
        .pragma_update(None, "user_version", 3)
        .expect("newer version");

    assert!(matches!(
        open_db(&cache),
        Err(CoreError::DatabaseSchema { schema: 3 })
    ));
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version");
    assert_eq!(version, 3);
    let tables: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |row| row.get(0),
        )
        .expect("unchanged database");
    assert_eq!(tables, 0);
}

#[test]
fn a_failed_migration_preserves_the_previous_schema_and_metadata() {
    let cache = cache("failed-migration");
    std::fs::create_dir_all(cache.root()).expect("cache directory");
    let connection = rusqlite::Connection::open(db_path(&cache)).expect("database");
    connection
        .execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
         INSERT INTO meta (key, value) VALUES ('last_played_folder', 'survival');
         CREATE TABLE download_jobs (existing TEXT);
         INSERT INTO download_jobs VALUES ('preserve me');
         PRAGMA user_version = 1;",
        )
        .expect("conflicting schema");

    assert!(matches!(open_db(&cache), Err(CoreError::Database(_))));
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version");
    assert_eq!(version, 1);
    let metadata: String = connection
        .query_row(
            "SELECT value FROM meta WHERE key = 'last_played_folder'",
            [],
            |row| row.get(0),
        )
        .expect("metadata");
    let existing: String = connection
        .query_row("SELECT existing FROM download_jobs", [], |row| row.get(0))
        .expect("preserved table");
    assert_eq!(metadata, "survival");
    assert_eq!(existing, "preserve me");
}
