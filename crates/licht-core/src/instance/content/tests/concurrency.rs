use super::*;

fn simulated_resolution(index: usize) -> ContentResolution {
    ContentResolution {
        index,
        entry: InstanceContentEntry {
            name: format!("mod-{index}.jar"),
            kind: InstanceContentKind::Mods,
            enabled: true,
            size: 1,
            title: format!("Resolved {index}"),
            version: Some(index.to_string()),
            icon_url: None,
        },
        fingerprint: ContentFingerprint {
            relative_path: Path::new("mods")
                .join(format!("mod-{index}.jar"))
                .to_string_lossy()
                .into_owned(),
            size: 1,
            modified: "100:0".to_string(),
            sha1: sha1_hex(&[index as u8]),
        },
    }
}

#[tokio::test]
async fn concurrent_content_limits_in_flight_lookups_and_publishes_completion_order() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let (mut releases, receivers): (Vec<_>, Vec<_>) = (0..6)
        .map(|_| {
            let (send, receive) = tokio::sync::oneshot::channel();
            (Some(send), receive)
        })
        .unzip();
    let lookups = receivers.into_iter().enumerate().map(|(index, gate)| {
        let active = Arc::clone(&active);
        let peak = Arc::clone(&peak);
        let started = started.clone();
        async move {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(count, Ordering::SeqCst);
            started.send(index).expect("lookup started");
            gate.await.expect("release lookup");
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(simulated_resolution(index))
        }
    });
    let mut entries: Vec<_> = (0..6)
        .map(|index| simulated_resolution(index).entry)
        .collect();
    let mut batches = Vec::new();
    let mut publish = |batch: &[InstanceContentEntry]| {
        batches.push(
            batch
                .iter()
                .map(|entry| entry.name.clone())
                .collect::<Vec<_>>(),
        );
        Ok(())
    };
    let resolve = enrich_content_entries(
        &mut entries,
        lookups,
        &mut publish,
        std::time::Instant::now() - std::time::Duration::from_millis(100),
    );
    let control = async {
        let mut first_wave = Vec::new();
        for _ in 0..4 {
            first_wave.push(starts.recv().await.expect("first wave"));
        }
        first_wave.sort();
        assert_eq!(first_wave, [0, 1, 2, 3]);
        assert!(
            starts.try_recv().is_err(),
            "a fifth lookup must wait for a free slot"
        );
        releases[2]
            .take()
            .expect("second gate")
            .send(())
            .expect("release second");
        assert_eq!(starts.recv().await, Some(4));
        releases[3]
            .take()
            .expect("third gate")
            .send(())
            .expect("release third");
        assert_eq!(starts.recv().await, Some(5));
        for index in [0, 1, 4, 5] {
            releases[index]
                .take()
                .expect("remaining gate")
                .send(())
                .expect("release remaining");
        }
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(resolve, control)
    })
    .await
    .expect("concurrent lookups must make progress");
    let fingerprints = result.expect("resolved content");
    assert_eq!(peak.load(Ordering::SeqCst), 4);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(
        fingerprints[0].relative_path,
        simulated_resolution(2).fingerprint.relative_path
    );
    assert_eq!(batches[0], ["mod-2.jar"]);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.version.as_deref())
            .collect::<Vec<_>>(),
        [
            Some("0"),
            Some("1"),
            Some("2"),
            Some("3"),
            Some("4"),
            Some("5")
        ]
    );
}

#[tokio::test]
async fn concurrent_content_propagates_lookup_errors_without_losing_published_results() {
    let mut entries: Vec<_> = (0..3)
        .map(|index| {
            let mut entry = simulated_resolution(index).entry;
            entry.version = None;
            entry
        })
        .collect();
    let lookups = (0..3).map(|index| async move {
        if index == 1 {
            tokio::task::yield_now().await;
            return Err(std::io::Error::other("lookup failed").into());
        }
        if index == 2 {
            std::future::pending::<()>().await;
        }
        Ok(simulated_resolution(index))
    });
    let mut batches = Vec::new();
    let result = enrich_content_entries(
        &mut entries,
        lookups,
        &mut |batch| {
            batches.push(batch.to_vec());
            Ok(())
        },
        std::time::Instant::now() - std::time::Duration::from_millis(100),
    )
    .await;
    assert!(matches!(result, Err(CoreError::Io(error)) if error.to_string() == "lookup failed"));
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0][0].version.as_deref(), Some("0"));
    assert_eq!(entries[0].version.as_deref(), Some("0"));
    assert_eq!(entries[1].version, None);
}

#[tokio::test]
async fn concurrent_content_preserves_cached_metadata_for_each_original_file() {
    let cache = cache("concurrent-cache");
    let root = instance(&cache);
    for index in 0..8 {
        let body = format!("cached file {index}");
        fs::write(
            root.join("mods").join(format!("mod-{index}.jar")),
            body.as_bytes(),
        )
        .expect("mod");
        seed_metadata(&cache, body.as_bytes(), &format!("Cached title {index}"));
    }
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    let entries = list_instance_content_with_metadata(
        &cache,
        &service.client(),
        &offline_client(),
        "classic",
        false,
    )
    .await
    .expect("cached content");
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.name, format!("mod-{index}.jar"));
        assert_eq!(entry.title, format!("Cached title {index}"));
    }
    let index = service
        .client()
        .content_fingerprints(&cache, "classic")
        .await
        .expect("fingerprints");
    assert_eq!(index.len(), 8);
    for (number, fingerprint) in index.iter().enumerate() {
        assert_eq!(
            fingerprint.sha1,
            sha1_hex(format!("cached file {number}").as_bytes())
        );
    }
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn concurrent_content_measures_simulated_metadata_latency() {
    let lookups = || {
        (0..8).map(|index| async move {
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            Ok(simulated_resolution(index))
        })
    };
    let sequential_start = std::time::Instant::now();
    let mut sequential = Vec::new();
    for lookup in lookups() {
        sequential.push(lookup.await.expect("sequential lookup").entry);
    }
    let sequential_time = sequential_start.elapsed();
    let concurrent_start = std::time::Instant::now();
    let mut concurrent: Vec<_> = (0..8)
        .map(|index| simulated_resolution(index).entry)
        .collect();
    enrich_content_entries(
        &mut concurrent,
        lookups(),
        &mut |_| Ok(()),
        std::time::Instant::now(),
    )
    .await
    .expect("concurrent lookups");
    let concurrent_time = concurrent_start.elapsed();
    assert_eq!(sequential, concurrent);
    eprintln!(
        "Simulated metadata (8 lookups, 40 ms each): sequential {sequential_time:?}, concurrent {concurrent_time:?}"
    );
}
