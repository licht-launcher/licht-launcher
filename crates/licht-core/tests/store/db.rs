use licht_core::{
    CoreError, SharedCache, clear_last_played_folder, db_path, last_played_folder, open_db,
    set_last_played_folder,
};
use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};

fn cache(name: &str) -> SharedCache {
    let root = std::env::temp_dir().join(format!("licht-db-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    SharedCache::at(root)
}

async fn raw_database(cache: &SharedCache) -> SqliteConnection {
    tokio::fs::create_dir_all(cache.root())
        .await
        .expect("directory");
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(db_path(cache))
            .create_if_missing(true),
    )
    .await
    .expect("raw database")
}

async fn scalar(connection: &mut SqliteConnection, query: &'static str) -> i64 {
    sqlx::query_scalar(query)
        .fetch_one(connection)
        .await
        .expect("scalar query")
}

#[tokio::test]
async fn open_creates_the_database_and_is_idempotent() {
    let cache = cache("open");
    open_db(&cache)
        .await
        .expect("open")
        .close()
        .await
        .expect("close");
    assert!(db_path(&cache).is_file());
    open_db(&cache)
        .await
        .expect("reopen")
        .close()
        .await
        .expect("close");
}

#[tokio::test]
async fn opening_an_up_to_date_database_can_read_while_another_connection_is_writing() {
    let cache = cache("open-during-write");
    set_last_played_folder(&cache, "survival")
        .await
        .expect("committed metadata");
    let mut connection = open_db(&cache).await.expect("writer connection");
    let mut transaction = connection
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("writer transaction");
    sqlx::query("UPDATE meta SET value = 'creative' WHERE key = 'last_played_folder'")
        .execute(&mut *transaction)
        .await
        .expect("pending write");
    let mut reader = open_db(&cache)
        .await
        .expect("opening a reader must not require a writer lock");
    let folder: String =
        sqlx::query_scalar("SELECT value FROM meta WHERE key = 'last_played_folder'")
            .fetch_one(&mut reader)
            .await
            .expect("committed metadata");
    assert_eq!(folder, "survival");
    reader.close().await.expect("close reader");
    transaction.commit().await.expect("commit writer");
    assert_eq!(
        last_played_folder(&cache).await.expect("updated"),
        Some("creative".into())
    );
    connection.close().await.expect("close writer");
}

#[tokio::test]
async fn concurrent_first_opens_share_a_complete_migrated_schema() {
    let cache = cache("concurrent-first-open");
    let barrier = tokio::sync::Barrier::new(4);
    futures_util::future::join_all((0..4).map(|_| async {
        barrier.wait().await;
        let mut connection = open_db(&cache).await.expect("concurrent open");
        assert_eq!(scalar(&mut connection, "PRAGMA user_version").await, 4);
        assert_eq!(scalar(&mut connection, "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('meta', 'download_jobs', 'download_job_artifacts')").await, 3);
        assert_artifact_path_lookup_is_indexed(&mut connection).await;
        connection.close().await.expect("close");
    })).await;
}

#[tokio::test]
async fn artifact_cleanup_seeks_by_path_on_new_and_reopened_databases() {
    let cache = cache("artifact-path-lookup");
    for _ in 0..2 {
        let mut connection = open_db(&cache).await.expect("open");
        assert_artifact_path_lookup_is_indexed(&mut connection).await;
        connection.close().await.expect("close");
    }
}

async fn assert_artifact_path_lookup_is_indexed(connection: &mut SqliteConnection) {
    let rows = sqlx::query("EXPLAIN QUERY PLAN SELECT artifact.path FROM download_job_artifacts artifact WHERE artifact.job_id = ?1 AND NOT EXISTS (SELECT 1 FROM download_job_artifacts other JOIN download_jobs job ON job.id = other.job_id WHERE other.path = artifact.path AND other.job_id != ?1 AND job.status NOT IN (?2, ?3))")
        .bind(42_i64).bind("\"completed\"").bind("\"cancelled\"").fetch_all(connection).await.expect("query plan");
    let plan: Vec<String> = rows
        .into_iter()
        .map(|row| row.try_get(3).expect("plan detail"))
        .collect();
    assert!(
        plan.iter()
            .any(|detail| detail.starts_with("SEARCH other ") && detail.contains("path=?")),
        "cleanup must seek by path rather than scan other artifacts: {plan:?}"
    );
}

#[tokio::test]
async fn last_played_folder_round_trips() {
    let cache = cache("last-played");
    assert_eq!(last_played_folder(&cache).await.expect("empty"), None);
    set_last_played_folder(&cache, "classic")
        .await
        .expect("set");
    assert_eq!(
        last_played_folder(&cache).await.expect("read").as_deref(),
        Some("classic")
    );
    set_last_played_folder(&cache, "survival")
        .await
        .expect("update");
    assert_eq!(
        last_played_folder(&cache).await.expect("read").as_deref(),
        Some("survival")
    );
    clear_last_played_folder(&cache).await.expect("clear");
    assert_eq!(last_played_folder(&cache).await.expect("cleared"), None);
}

#[tokio::test]
async fn a_version_one_database_gains_download_jobs_without_losing_existing_metadata() {
    let cache = cache("downloads-migration");
    let mut connection = raw_database(&cache).await;
    sqlx::raw_sql("CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL); INSERT INTO meta VALUES ('last_played_folder', 'survival'); PRAGMA user_version = 1;").execute(&mut connection).await.expect("legacy schema");
    connection.close().await.expect("close");
    let mut connection = open_db(&cache).await.expect("migrate");
    assert_eq!(
        scalar(
            &mut connection,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'download_jobs'"
        )
        .await,
        1
    );
    assert_eq!(
        last_played_folder(&cache).await.expect("metadata"),
        Some("survival".into())
    );
    assert_eq!(scalar(&mut connection, "PRAGMA user_version").await, 4);
    connection.close().await.expect("close");
}

#[tokio::test]
async fn a_newer_database_schema_is_rejected_without_downgrading_it() {
    let cache = cache("future-schema");
    let mut connection = raw_database(&cache).await;
    sqlx::query("PRAGMA user_version = 5")
        .execute(&mut connection)
        .await
        .expect("future schema");
    assert!(matches!(
        open_db(&cache).await,
        Err(CoreError::DatabaseSchema { schema: 5 })
    ));
    assert_eq!(scalar(&mut connection, "PRAGMA user_version").await, 5);
    assert_eq!(
        scalar(
            &mut connection,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'"
        )
        .await,
        0
    );
    connection.close().await.expect("close");
}

#[tokio::test]
async fn a_failed_migration_preserves_the_previous_schema_and_metadata() {
    let cache = cache("failed-migration");
    let mut connection = raw_database(&cache).await;
    sqlx::raw_sql("CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL); INSERT INTO meta VALUES ('last_played_folder', 'survival'); CREATE TABLE download_jobs (existing TEXT); INSERT INTO download_jobs VALUES ('preserve me'); PRAGMA user_version = 1;").execute(&mut connection).await.expect("conflicting schema");
    assert!(matches!(open_db(&cache).await, Err(CoreError::Database(_))));
    assert_eq!(scalar(&mut connection, "PRAGMA user_version").await, 1);
    let metadata: String =
        sqlx::query_scalar("SELECT value FROM meta WHERE key = 'last_played_folder'")
            .fetch_one(&mut connection)
            .await
            .expect("metadata");
    let existing: String = sqlx::query_scalar("SELECT existing FROM download_jobs")
        .fetch_one(&mut connection)
        .await
        .expect("preserved table");
    assert_eq!(metadata, "survival");
    assert_eq!(existing, "preserve me");
    connection.close().await.expect("close");
}

#[tokio::test]
async fn version_two_jobs_keep_their_identity_and_progress_when_targets_become_optional() {
    let cache = cache("version-two-jobs");
    let game = SharedCache::at(cache.root().join("data"));
    let mut connection = raw_database(&cache).await;
    sqlx::raw_sql("CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL); INSERT INTO meta VALUES ('last_played_folder', 'Survival'); CREATE TABLE download_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, data_directory TEXT NOT NULL, instance_folder TEXT NOT NULL, version_id TEXT NOT NULL, operation TEXT NOT NULL, status TEXT NOT NULL, component TEXT, phase TEXT NOT NULL, progress TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL); PRAGMA user_version = 2;").execute(&mut connection).await.expect("legacy schema");
    let progress = licht_core::DownloadProgress {
        finished: 0,
        failed: 0,
        total: 1,
        bytes_done: 5,
        bytes_total: 11,
    };
    sqlx::query(
        "INSERT INTO download_jobs VALUES (42, ?1, 'Survival', '1.21.1', ?2, ?3, ?4, ?5, ?6, 1, 2)",
    )
    .bind(game.root().to_str().expect("path"))
    .bind("\"install\"")
    .bind("\"running\"")
    .bind("\"java\"")
    .bind("\"downloading\"")
    .bind(serde_json::to_string(&progress).expect("progress"))
    .execute(&mut connection)
    .await
    .expect("legacy job");
    connection.close().await.expect("close");
    let job = licht_core::get_download_job(&cache, &game, "42")
        .await
        .expect("migrated job");
    assert_eq!(job.progress, progress);
    assert_eq!(job.instance_folder.as_deref(), Some("Survival"));
    assert_eq!(job.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(job.status, licht_core::DownloadJobStatus::Running);
    assert_eq!(job.java_major, None);
    assert_eq!(
        last_played_folder(&cache).await.expect("metadata"),
        Some("Survival".into())
    );
    let pending = licht_core::create_pending_download_job(
        &cache,
        &game,
        "Pending",
        None,
        licht_core::DownloadJobOperation::Install,
    )
    .await
    .expect("unresolved version");
    assert!(pending.id.parse::<u64>().expect("id") > 42);
    assert_eq!(pending.version_id, None);
}

#[tokio::test]
async fn version_three_gains_indexed_artifact_lookup_without_changing_stored_data() {
    let cache = cache("version-three-artifacts");
    let game = SharedCache::at(cache.root().join("data"));
    let mut connection = raw_database(&cache).await;
    sqlx::raw_sql("CREATE TABLE meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL); INSERT INTO meta VALUES ('last_played_folder', 'Survival'); CREATE TABLE download_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, data_directory TEXT NOT NULL, instance_folder TEXT, version_id TEXT, operation TEXT NOT NULL, status TEXT NOT NULL, component TEXT, phase TEXT NOT NULL, progress TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, error TEXT, java_major INTEGER); CREATE INDEX download_jobs_directory ON download_jobs (data_directory, id); CREATE TABLE download_job_artifacts (job_id INTEGER NOT NULL, path TEXT NOT NULL, PRIMARY KEY(job_id, path)); PRAGMA user_version = 3;").execute(&mut connection).await.expect("legacy schema");
    let progress = licht_core::DownloadProgress {
        finished: 0,
        failed: 0,
        total: 1,
        bytes_done: 5,
        bytes_total: 11,
    };
    sqlx::query("INSERT INTO download_jobs VALUES (42, ?1, 'Survival', NULL, ?2, ?3, ?4, ?5, ?6, 1, 2, NULL, NULL)").bind(game.root().to_str().expect("path")).bind("\"install\"").bind("\"paused\"").bind("\"modpack\"").bind("\"downloading\"").bind(serde_json::to_string(&progress).expect("progress")).execute(&mut connection).await.expect("legacy job");
    sqlx::raw_sql(
        "INSERT INTO download_job_artifacts VALUES (42, 'shared.jar'), (42, 'exclusive.jar');",
    )
    .execute(&mut connection)
    .await
    .expect("associations");
    connection.close().await.expect("close");
    let mut connection = open_db(&cache).await.expect("migrate");
    assert_artifact_path_lookup_is_indexed(&mut connection).await;
    assert_eq!(scalar(&mut connection, "PRAGMA user_version").await, 4);
    let associations: Vec<(i64, String)> =
        sqlx::query_as("SELECT job_id, path FROM download_job_artifacts ORDER BY path")
            .fetch_all(&mut connection)
            .await
            .expect("association rows");
    assert_eq!(
        associations,
        vec![(42, "exclusive.jar".into()), (42, "shared.jar".into())]
    );
    connection.close().await.expect("close");
    let job = licht_core::get_download_job(&cache, &game, "42")
        .await
        .expect("preserved job");
    assert_eq!(job.progress, progress);
    assert_eq!(job.status, licht_core::DownloadJobStatus::Paused);
    assert_eq!(job.instance_folder.as_deref(), Some("Survival"));
    assert_eq!(job.version_id, None);
    assert_eq!((job.created_at, job.updated_at), (1, 2));
    assert_eq!(
        last_played_folder(&cache).await.expect("metadata"),
        Some("Survival".into())
    );
    assert_eq!(
        licht_core::get_download_job(&cache, &game, "42")
            .await
            .expect("reopened job"),
        job
    );
}
