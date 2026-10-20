use super::*;

#[tokio::test]
async fn progressive_content_publishes_local_rows_before_database_failure() {
    let cache = cache("progressive-before-database");
    let root = instance(&cache);
    fs::write(root.join("mods").join("one.jar"), b"mod").expect("mod");
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    let database = service.client();
    service.shutdown().await.expect("shutdown");
    let mut snapshots = Vec::new();
    let result = list_instance_content_progressively(
        &cache,
        &database,
        &offline_client(),
        "classic",
        false,
        |entries| {
            snapshots.push(entries.to_vec());
            Ok(())
        },
    )
    .await;
    assert!(matches!(result, Err(CoreError::DatabaseService { .. })));
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0][0].name, "one.jar");
    assert_eq!(snapshots[0][0].version, None);
}

#[tokio::test]
async fn progressive_content_propagates_publication_failure() {
    let cache = cache("progressive-emission-error");
    instance(&cache);
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    let result = list_instance_content_progressively(
        &cache,
        &service.client(),
        &offline_client(),
        "classic",
        false,
        |_| Err(std::io::Error::other("event bridge unavailable").into()),
    )
    .await;
    assert!(
        matches!(result, Err(CoreError::Io(error)) if error.to_string() == "event bridge unavailable")
    );
    service.shutdown().await.expect("shutdown");
}

#[test]
fn progressive_content_batches_are_throttled_and_errors_propagate() {
    let start = std::time::Instant::now();
    let mut last = start;
    let mut snapshots = Vec::new();
    let mut publish = |entries: &[InstanceContentEntry]| {
        snapshots.push(entries.len());
        Ok(())
    };
    assert!(
        !publish_content_batch(
            &[],
            &mut last,
            start + std::time::Duration::from_millis(99),
            &mut publish
        )
        .expect("throttled")
    );
    assert!(
        publish_content_batch(
            &[],
            &mut last,
            start + std::time::Duration::from_millis(100),
            &mut publish
        )
        .expect("publish")
    );
    assert!(
        !publish_content_batch(
            &[],
            &mut last,
            start + std::time::Duration::from_millis(199),
            &mut publish
        )
        .expect("throttled")
    );
    assert!(
        publish_content_batch(
            &[],
            &mut last,
            start + std::time::Duration::from_millis(200),
            &mut publish
        )
        .expect("publish")
    );
    assert_eq!(snapshots, [0, 0]);
    assert!(
        publish_content_batch(
            &[],
            &mut last,
            start + std::time::Duration::from_millis(300),
            &mut |_| Err(std::io::Error::other("publish failed").into())
        )
        .is_err()
    );
}

#[tokio::test]
async fn progressive_content_measures_first_list_and_completion_with_local_fixtures() {
    let cache = cache("progressive-latency");
    let root = instance(&cache);
    for index in 0..256 {
        fs::write(
            root.join("mods").join(format!("mod-{index}.jar")),
            b"fixture",
        )
        .expect("mod");
    }
    seed_metadata(&cache, b"fixture", "Known metadata");
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    for pass in ["cold fingerprint index", "warm fingerprint index"] {
        let start = std::time::Instant::now();
        let mut first = None;
        let mut published = 0;
        let entries = list_instance_content_progressively(
            &cache,
            &service.client(),
            &offline_client(),
            "classic",
            false,
            |entries| {
                if first.is_none() {
                    first = Some(start.elapsed());
                    assert_eq!(entries.len(), 256);
                    assert!(entries.iter().all(|entry| entry.version.is_none()));
                }
                published += 1;
                Ok(())
            },
        )
        .await
        .expect("enriched content");
        let total = start.elapsed();
        let first = first.expect("initial list");
        assert!(first < total);
        assert!(entries.iter().all(
            |entry| entry.title == "Known metadata" && entry.version.as_deref() == Some("1.0")
        ));
        eprintln!(
            "Content fixture (256 files, {pass}): first list {first:?}, complete {total:?}, events {published}"
        );
    }
    service.shutdown().await.expect("shutdown");
}
