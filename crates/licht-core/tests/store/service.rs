use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;

use licht_core::{
    CoreError, DatabaseService, DownloadJobOperation, DownloadJobStatus, DownloadJobUpdate,
    DownloadPhase, DownloadProgress, SharedCache, db_path, open_db,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    cache: SharedCache,
}

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::SeqCst);
        Self {
            cache: SharedCache::at(std::env::temp_dir().join(format!(
                "licht-database-service-{}-{sequence}",
                std::process::id()
            ))),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(self.cache.root())
            && error.kind() != std::io::ErrorKind::NotFound
            && !std::thread::panicking()
        {
            panic!("fixture cleanup failed: {error}");
        }
    }
}

async fn assert_pending(mut future: std::pin::Pin<&mut impl Future>) {
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

async fn hold_writer(cache: &SharedCache) -> SqliteConnection {
    let mut connection = open_db(cache).await.expect("writer connection");
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut connection)
        .await
        .expect("writer lock");
    connection
}

async fn release_writer(mut connection: SqliteConnection) {
    sqlx::query("ROLLBACK")
        .execute(&mut connection)
        .await
        .expect("release writer lock");
    connection.close().await.expect("close writer");
}

#[tokio::test]
async fn metadata_round_trips_and_survives_service_restart() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    assert_eq!(client.last_played_folder().await.expect("empty"), None);
    client
        .set_last_played_folder(" survival ")
        .await
        .expect("set");
    assert_eq!(
        client.last_played_folder().await.expect("read"),
        Some("survival".into())
    );
    client.clear_last_played_folder().await.expect("clear");
    assert_eq!(client.last_played_folder().await.expect("cleared"), None);
    client
        .set_last_played_folder("creative")
        .await
        .expect("persist");
    service.shutdown().await.expect("shutdown");

    let restarted = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("restart");
    assert_eq!(
        restarted
            .client()
            .last_played_folder()
            .await
            .expect("stored"),
        Some("creative".into())
    );
    restarted
        .shutdown()
        .await
        .expect("shutdown restarted service");
}

#[tokio::test]
async fn invalid_input_does_not_change_metadata_or_stop_the_service() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    client
        .set_last_played_folder("survival")
        .await
        .expect("set");
    assert!(matches!(
        client.set_last_played_folder(" \t ").await,
        Err(CoreError::InstanceName)
    ));
    assert_eq!(
        client.last_played_folder().await.expect("unchanged"),
        Some("survival".into())
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn a_sql_error_is_returned_and_the_service_accepts_subsequent_requests() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let mut connection = open_db(&fixture.cache).await.expect("fixture connection");
    sqlx::raw_sql(
        "CREATE TRIGGER reject_metadata BEFORE INSERT ON meta
         BEGIN SELECT RAISE(FAIL, 'fixture write failure'); END;",
    )
    .execute(&mut connection)
    .await
    .expect("reject writes");

    assert!(matches!(
        client.set_last_played_folder("survival").await,
        Err(CoreError::Database(_))
    ));
    assert_eq!(
        client
            .last_played_folder()
            .await
            .expect("read after failure"),
        None
    );
    sqlx::query("DROP TRIGGER reject_metadata;")
        .execute(&mut connection)
        .await
        .expect("allow writes");
    connection.close().await.expect("close fixture connection");
    client
        .set_last_played_folder("creative")
        .await
        .expect("write after failure");
    assert_eq!(
        client.last_played_folder().await.expect("stored"),
        Some("creative".into())
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn startup_reports_an_unsupported_schema_without_modifying_it() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.cache.root()).expect("fixture directory");
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(db_path(&fixture.cache))
            .create_if_missing(true),
    )
    .await
    .expect("future database");
    sqlx::query("PRAGMA user_version = 11")
        .execute(&mut connection)
        .await
        .expect("future schema");
    connection.close().await.expect("close future database");

    assert!(matches!(
        DatabaseService::start(fixture.cache.clone()).await,
        Err(CoreError::DatabaseSchema { schema: 11 })
    ));
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(db_path(&fixture.cache)),
    )
    .await
    .expect("unchanged database");
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut connection)
        .await
        .expect("schema version");
    assert_eq!(version, 11);
    connection.close().await.expect("close fixture connection");
}

#[tokio::test]
async fn startup_reports_an_invalid_database_directory() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.cache.root()).expect("fixture directory");
    let blocked = fixture.cache.root().join("blocked");
    std::fs::write(&blocked, b"not a directory").expect("blocking file");
    assert!(matches!(
        DatabaseService::start(SharedCache::at(blocked)).await,
        Err(CoreError::Io(_))
    ));
}

#[tokio::test]
async fn a_full_queue_keeps_the_runtime_responsive_and_preserves_write_order() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let writer = hold_writer(&fixture.cache).await;
    let writes = (0..130).map(|index| {
        let client = client.clone();
        async move {
            client
                .set_last_played_folder(&format!("world-{index}"))
                .await
        }
    });
    let mut writes = Box::pin(futures_util::future::join_all(writes));
    assert_pending(writes.as_mut()).await;

    tokio::task::yield_now().await;
    release_writer(writer).await;
    for result in writes.await {
        result.expect("queued write");
    }
    assert_eq!(
        client.last_played_folder().await.expect("last write"),
        Some("world-129".into())
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn shutdown_drains_a_full_queue_and_rejects_requests_still_waiting_for_capacity() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let writer = hold_writer(&fixture.cache).await;
    let writes = (0..130).map(|index| {
        let client = client.clone();
        async move {
            client
                .set_last_played_folder(&format!("world-{index}"))
                .await
        }
    });
    let mut writes = Box::pin(futures_util::future::join_all(writes));
    assert_pending(writes.as_mut()).await;
    let mut shutdown = Box::pin(service.shutdown());
    assert_pending(shutdown.as_mut()).await;

    release_writer(writer).await;
    let (closed, results) = tokio::join!(shutdown, writes);
    closed.expect("drain the full queue and close");
    let mut accepted = 0;
    for (index, result) in results.into_iter().enumerate() {
        match result {
            Ok(()) => {
                assert_eq!(
                    index, accepted,
                    "accepted writes must form an ordered prefix"
                );
                accepted += 1;
            }
            Err(CoreError::DatabaseService { .. }) => {}
            Err(error) => panic!("unexpected write error: {error}"),
        }
    }
    // At most one write is in flight in addition to the 64 pending queue slots.
    assert!((64..=65).contains(&accepted), "accepted {accepted} writes");
    assert_eq!(
        licht_core::last_played_folder(&fixture.cache)
            .await
            .expect("drained metadata"),
        Some(format!("world-{}", accepted - 1))
    );
}

#[tokio::test]
async fn shutdown_drains_an_accepted_write_after_its_caller_disconnects() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let writer = hold_writer(&fixture.cache).await;
    let mut write = Box::pin(client.set_last_played_folder("survival"));
    assert_pending(write.as_mut()).await;
    drop(write);

    let mut shutdown = Box::pin(service.shutdown());
    assert_pending(shutdown.as_mut()).await;
    assert!(matches!(
        client.last_played_folder().await,
        Err(CoreError::DatabaseService { .. })
    ));
    release_writer(writer).await;
    shutdown.await.expect("drain and close");
    assert_eq!(
        licht_core::last_played_folder(&fixture.cache)
            .await
            .expect("committed write"),
        Some("survival".into())
    );
    assert!(matches!(
        client.clear_last_played_folder().await,
        Err(CoreError::DatabaseService { .. })
    ));
}

fn job_update(status: DownloadJobStatus) -> DownloadJobUpdate {
    DownloadJobUpdate {
        status,
        component: None,
        phase: DownloadPhase::Preparing,
        progress: DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        },
        error: None,
    }
}

#[tokio::test]
async fn account_identifiers_preserve_legacy_metadata_and_do_not_lose_concurrent_updates() {
    let fixture = Fixture::new();
    let mut connection = open_db(&fixture.cache).await.expect("database");
    licht_core::set_meta(
        &mut connection,
        "microsoft_account_uuids",
        " legacy-first ,, legacy-second ",
    )
    .await
    .expect("legacy identifiers");
    connection.close().await.expect("close fixture connection");
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    assert_eq!(
        client
            .remembered_account_uuids()
            .await
            .expect("legacy metadata"),
        vec!["legacy-first", "legacy-second"]
    );
    let updates = (0..8).map(|index| {
        let client = client.clone();
        async move {
            client
                .remember_account_uuid(&format!("fixture-account-{index}"), true)
                .await
        }
    });
    for result in futures_util::future::join_all(updates).await {
        result.expect("remember account");
    }
    client
        .remember_account_uuid("legacy-first", false)
        .await
        .expect("forget account");
    client
        .remember_account_uuid("legacy-second", true)
        .await
        .expect("remember duplicate");
    let mut actual = client
        .remembered_account_uuids()
        .await
        .expect("all identifiers");
    actual.sort();
    let mut expected: Vec<String> = (0..8)
        .map(|index| format!("fixture-account-{index}"))
        .collect();
    expected.push("legacy-second".into());
    expected.sort();
    assert_eq!(actual, expected);
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn download_requests_preserve_lifecycle_version_recovery_and_transition_validation() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let game = SharedCache::at(fixture.cache.root().join("game/data"));
    let paused = client
        .create_pending_download_job(&game, "Paused", None, DownloadJobOperation::Install)
        .await
        .expect("create");
    client
        .set_download_job_version(&game, &paused.id, "1.21.1")
        .await
        .expect("resolve version");
    client
        .update_download_job(&game, &paused.id, job_update(DownloadJobStatus::Paused))
        .await
        .expect("pause");
    let running = client
        .create_pending_download_job(
            &game,
            "Running",
            Some("1.21.1"),
            DownloadJobOperation::Repair,
        )
        .await
        .expect("create running");
    client
        .update_download_job(&game, &running.id, job_update(DownloadJobStatus::Running))
        .await
        .expect("run");
    let queued = client
        .create_pending_download_job(&game, "Queued", None, DownloadJobOperation::Install)
        .await
        .expect("create queued");

    assert_eq!(
        client.recover_download_jobs(&game).await.expect("recover"),
        2
    );
    let stored = client
        .get_download_job(&game, &paused.id)
        .await
        .expect("paused job");
    assert_eq!(stored.status, DownloadJobStatus::Paused);
    assert_eq!(stored.version_id.as_deref(), Some("1.21.1"));
    for id in [&running.id, &queued.id] {
        assert_eq!(
            client
                .get_download_job(&game, id)
                .await
                .expect("interrupted")
                .status,
            DownloadJobStatus::Interrupted
        );
    }
    assert_eq!(
        client.recover_download_jobs(&game).await.expect("repeat"),
        0
    );
    client
        .update_download_job(&game, &running.id, job_update(DownloadJobStatus::Running))
        .await
        .expect("resume");
    client
        .update_download_job(&game, &running.id, job_update(DownloadJobStatus::Completed))
        .await
        .expect("complete");
    assert_eq!(
        client
            .list_download_jobs(&game)
            .await
            .expect("unfinished")
            .len(),
        2
    );
    assert!(matches!(
        client
            .update_download_job(&game, &running.id, job_update(DownloadJobStatus::Running))
            .await,
        Err(CoreError::DownloadJobTransition)
    ));
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn download_requests_isolate_data_directories_and_validate_targets() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let first = SharedCache::at(fixture.cache.root().join("first/data"));
    let second = SharedCache::at(fixture.cache.root().join("second/data"));
    let job = client
        .create_pending_download_job(&first, "Survival", None, DownloadJobOperation::Install)
        .await
        .expect("create");
    assert!(
        client
            .list_download_jobs(&second)
            .await
            .expect("other directory")
            .is_empty()
    );
    assert!(matches!(
        client.get_download_job(&second, &job.id).await,
        Err(CoreError::DownloadJobMissing)
    ));
    assert_eq!(
        client
            .recover_download_jobs(&second)
            .await
            .expect("other recovery"),
        0
    );
    assert!(matches!(
        client
            .create_pending_download_job(&first, "../escape", None, DownloadJobOperation::Install)
            .await,
        Err(CoreError::CachePath)
    ));
    assert!(matches!(
        client
            .list_download_jobs(&SharedCache::at("relative"))
            .await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
    assert_eq!(
        client
            .get_download_job(&first, &job.id)
            .await
            .expect("original")
            .status,
        DownloadJobStatus::Queued
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn artifact_requests_keep_shared_references_and_never_remove_files() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let game = SharedCache::at(fixture.cache.root().join("game/data"));
    let first = client
        .create_pending_download_job(&game, "First", None, DownloadJobOperation::Repair)
        .await
        .expect("first job");
    let second = client
        .create_pending_download_job(&game, "Second", None, DownloadJobOperation::Repair)
        .await
        .expect("second job");
    let shared = game.root().join("shared.jar");
    let exclusive = game.root().join("exclusive.jar");
    std::fs::create_dir_all(game.root()).expect("directory");
    let partial = exclusive.with_extension("jar.part");
    std::fs::write(&partial, b"partial").expect("partial");
    client
        .register_download_artifacts(&game, &first.id, &[shared.clone(), exclusive.clone()])
        .await
        .expect("first artifacts");
    client
        .register_download_artifacts(&game, &second.id, std::slice::from_ref(&shared))
        .await
        .expect("shared artifact");
    assert_eq!(
        client
            .download_cleanup_candidates(&game, &first.id)
            .await
            .expect("exclusive candidates"),
        vec![exclusive.clone()]
    );
    assert!(partial.exists(), "database requests must not remove files");
    client
        .update_download_job(&game, &second.id, job_update(DownloadJobStatus::Cancelled))
        .await
        .expect("cancel other job");
    let mut candidates = client
        .download_cleanup_candidates(&game, &first.id)
        .await
        .expect("released candidates");
    candidates.sort();
    let mut expected = vec![shared, exclusive];
    expected.sort();
    assert_eq!(candidates, expected);
    assert!(matches!(
        client
            .register_download_artifacts(
                &game,
                &first.id,
                &[game.root().join("valid.jar"), game.root().join("../escape")]
            )
            .await,
        Err(CoreError::CachePath)
    ));
    assert_eq!(
        client
            .download_cleanup_candidates(&game, &first.id)
            .await
            .expect("rolled back registration")
            .len(),
        2
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn java_download_requests_validate_the_major_and_keep_typed_targets() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let game = SharedCache::at(fixture.cache.root().join("game/data"));
    let java = client
        .create_java_download_job(&game, 21)
        .await
        .expect("Java job");
    assert_eq!(java.java_major, Some(21));
    assert_eq!(java.instance_folder, None);
    assert!(matches!(
        client.create_java_download_job(&game, 9).await,
        Err(CoreError::DownloadJobInvalid { .. })
    ));
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn a_download_write_waiting_for_sqlite_does_not_block_the_runtime() {
    let fixture = Fixture::new();
    let service = DatabaseService::start(fixture.cache.clone())
        .await
        .expect("start");
    let client = service.client();
    let game = SharedCache::at(fixture.cache.root().join("game/data"));
    let writer = hold_writer(&fixture.cache).await;
    let mut create = Box::pin(client.create_pending_download_job(
        &game,
        "Survival",
        None,
        DownloadJobOperation::Install,
    ));
    assert_pending(create.as_mut()).await;
    tokio::task::yield_now().await;
    release_writer(writer).await;
    let job = create.await.expect("create after releasing writer");
    assert_eq!(
        client
            .get_download_job(&game, &job.id)
            .await
            .expect("persisted job"),
        job
    );
    service.shutdown().await.expect("shutdown");
}
