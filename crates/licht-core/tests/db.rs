use licht_core::{
    SharedCache, clear_last_played_folder, db_path, last_played_folder, open_db,
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
