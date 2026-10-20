use super::*;

#[tokio::test]
async fn content_reuses_persisted_fingerprints_and_manual_refresh_revalidates_bytes() {
    let cache = cache("fingerprint-refresh");
    let root = instance(&cache);
    let path = root.join("mods").join("one.jar");
    fs::write(&path, b"old").expect("original mod");
    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("modified");
    seed_metadata(&cache, b"old", "Original");
    seed_metadata(&cache, b"new", "Updated");
    let client = offline_client();
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    let entries =
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
            .await
            .expect("initial content");
    assert_eq!(entries[0].title, "Original");
    service.shutdown().await.expect("shutdown");

    // Preserving size and mtime deliberately exercises the accepted cache heuristic.
    fs::write(&path, b"new").expect("same-size mod");
    set_modified(&path, modified);
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("restart");
    let entries =
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
            .await
            .expect("cached content");
    assert_eq!(
        entries[0].title, "Original",
        "unchanged attributes must reuse the persisted SHA1 rather than rehash bytes"
    );
    let entries =
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", true)
            .await
            .expect("manual refresh with cached metadata");
    assert_eq!(entries[0].title, "Updated");
    assert_eq!(
        service
            .client()
            .content_fingerprints(&cache, "classic")
            .await
            .expect("updated index")[0]
            .sha1,
        sha1_hex(b"new")
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn content_detects_added_resized_and_removed_files() {
    let cache = cache("fingerprint-changes");
    let root = instance(&cache);
    let path = root.join("mods").join("one.jar");
    let removed = root.join("mods").join("removed.jar");
    fs::write(&path, b"old").expect("original");
    fs::write(&removed, b"removed").expect("removed");
    for (body, title) in [
        (b"old".as_slice(), "Original"),
        (b"removed", "Removed"),
        (b"longer", "Resized"),
        (b"added", "Added"),
    ] {
        seed_metadata(&cache, body, title);
    }
    let client = offline_client();
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
        .await
        .expect("initial content");
    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("modified");
    fs::write(&path, b"longer").expect("resized");
    set_modified(&path, modified);
    fs::remove_file(&removed).expect("remove mod");
    fs::write(root.join("mods").join("added.jar"), b"added").expect("added mod");
    let entries =
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
            .await
            .expect("updated content");
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.title.as_str())
            .collect::<Vec<_>>(),
        ["Added", "Resized"]
    );
    let index = service
        .client()
        .content_fingerprints(&cache, "classic")
        .await
        .expect("index");
    assert_eq!(index.len(), 2);
    assert!(
        !index
            .iter()
            .any(|entry| entry.relative_path.ends_with("removed.jar"))
    );
    fs::remove_file(&path).expect("remove resized");
    fs::remove_file(root.join("mods").join("added.jar")).expect("remove added");
    assert!(
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
            .await
            .expect("empty content")
            .is_empty()
    );
    assert!(
        service
            .client()
            .content_fingerprints(&cache, "classic")
            .await
            .expect("empty index")
            .is_empty()
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn content_detects_mtime_changes_even_when_size_is_unchanged() {
    let cache = cache("fingerprint-mtime");
    let root = instance(&cache);
    let path = root.join("mods").join("one.jar");
    fs::write(&path, b"old").expect("original");
    set_modified(
        &path,
        UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
    );
    seed_metadata(&cache, b"old", "Original");
    seed_metadata(&cache, b"new", "Updated");
    let client = offline_client();
    let service = crate::DatabaseService::start(cache.clone())
        .await
        .expect("service");
    list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
        .await
        .expect("initial content");
    fs::write(&path, b"new").expect("same-size change");
    set_modified(
        &path,
        UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_100),
    );
    let entries =
        list_instance_content_with_metadata(&cache, &service.client(), &client, "classic", false)
            .await
            .expect("updated content");
    assert_eq!(entries[0].title, "Updated");
    service.shutdown().await.expect("shutdown");
}
