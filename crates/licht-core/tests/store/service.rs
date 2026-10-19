use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;

use licht_core::{CoreError, DatabaseService, SharedCache, db_path, open_db};

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

async fn hold_writer(
    cache: &SharedCache,
) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<bool>) {
    let cache = cache.clone();
    let (ready, locked) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut connection = open_db(&cache).expect("writer connection");
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .expect("writer lock");
        ready.send(()).expect("writer ready receiver");
        let acknowledged = released.recv_timeout(Duration::from_secs(10)).is_ok();
        transaction.rollback().expect("release writer lock");
        acknowledged
    });
    locked.await.expect("writer ready");
    (release, worker)
}

fn release_writer(release: std::sync::mpsc::Sender<()>, worker: std::thread::JoinHandle<bool>) {
    release.send(()).expect("writer release receiver");
    assert!(
        worker.join().expect("writer thread"),
        "writer release timed out"
    );
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
    let connection = open_db(&fixture.cache).expect("fixture connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_metadata BEFORE INSERT ON meta
         BEGIN SELECT RAISE(FAIL, 'fixture write failure'); END;",
        )
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
    connection
        .execute_batch("DROP TRIGGER reject_metadata;")
        .expect("allow writes");
    drop(connection);
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
    let connection = rusqlite::Connection::open(db_path(&fixture.cache)).expect("future database");
    connection
        .pragma_update(None, "user_version", 5)
        .expect("future schema");
    drop(connection);

    assert!(matches!(
        DatabaseService::start(fixture.cache.clone()).await,
        Err(CoreError::DatabaseSchema { schema: 5 })
    ));
    let connection =
        rusqlite::Connection::open(db_path(&fixture.cache)).expect("unchanged database");
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version");
    assert_eq!(version, 5);
    drop(connection);
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
    let (release, writer) = hold_writer(&fixture.cache).await;
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
    release_writer(release, writer);
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
    let (release, writer) = hold_writer(&fixture.cache).await;
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

    release_writer(release, writer);
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
        licht_core::last_played_folder(&fixture.cache).expect("drained metadata"),
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
    let (release, writer) = hold_writer(&fixture.cache).await;
    let mut write = Box::pin(client.set_last_played_folder("survival"));
    assert_pending(write.as_mut()).await;
    drop(write);

    let mut shutdown = Box::pin(service.shutdown());
    assert_pending(shutdown.as_mut()).await;
    assert!(matches!(
        client.last_played_folder().await,
        Err(CoreError::DatabaseService { .. })
    ));
    release_writer(release, writer);
    shutdown.await.expect("drain and close");
    assert_eq!(
        licht_core::last_played_folder(&fixture.cache).expect("committed write"),
        Some("survival".into())
    );
    assert!(matches!(
        client.clear_last_played_folder().await,
        Err(CoreError::DatabaseService { .. })
    ));
}
