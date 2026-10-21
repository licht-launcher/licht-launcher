use super::*;
use licht_core::{
    InstanceSource, SharedCache, configure_instance_version, create_pending_instance, open_db,
};
use sqlx::Connection;

fn settings() -> InstanceSettings {
    InstanceSettings {
        override_window: true,
        override_memory: true,
        override_java: true,
        override_jvm_arguments: true,
        java_path: Some("/custom/java".into()),
        max_memory_mb: 8192,
        jvm_arguments: vec!["-Dexample=with spaces".into(), "-Xss1M".into()],
        fullscreen: true,
        width: Some(1920),
        height: Some(1080),
    }
}

#[tokio::test]
async fn profiles_are_isolated_by_directory_and_ignore_obsolete_json() {
    let root = scratch("sqlite-isolation");
    let service = database(&root).await;
    let client = service.client();
    let first = root.join("instances");
    let second = root.join("other").join("instances");
    create_instance(&client, &first, "World", "1.20.1", &plain_template())
        .await
        .expect("first");
    let other = create_instance(&client, &second, "World", "1.21.1", &plain_template())
        .await
        .expect("second");
    rename_instance(&client, &first, "World", "Survival")
        .await
        .expect("rename");
    let obsolete = first.join("World").join("instance.json");
    fs::write(&obsolete, b"invalid obsolete profile").expect("obsolete file");
    assert_eq!(
        load_instance(&client, &first.join("World"))
            .await
            .expect("database profile")
            .name,
        "Survival"
    );
    let copy = duplicate_instance(&client, &first, "World", "Copy")
        .await
        .expect("copy");
    assert_eq!(copy.version_id.as_deref(), Some("1.20.1"));
    assert!(!first.join("Copy").join("instance.json").exists());
    assert_eq!(
        fs::read(obsolete).expect("untouched"),
        b"invalid obsolete profile"
    );
    assert_eq!(
        list_instances(&client, &second).await.expect("isolated"),
        vec![other]
    );
    fs::remove_dir_all(first.join("World")).expect("externally removed");
    assert_eq!(
        list_instances(&client, &first)
            .await
            .expect("missing folder omitted"),
        vec![copy]
    );
    delete_instance(&client, &first, "World")
        .await
        .expect("remove stale profile");
    let recreated = create_instance(&client, &first, "World", "1.21.1", &plain_template())
        .await
        .expect("recreate");
    assert_eq!(recreated.name, "World");
    service.shutdown().await.expect("shutdown");
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn settings_and_preparation_updates_preserve_each_other() {
    let root = scratch("sqlite-concurrent");
    let service = database(&root).await;
    let client = service.client();
    let instances = root.join("instances");
    let entry = create_pending_instance(
        &client,
        &instances,
        "World",
        InstanceSource::Vanilla {
            game_version: "1.20.1".into(),
        },
        &plain_template(),
    )
    .await
    .expect("pending");
    let options = settings();
    let (edited, prepared, renamed) = tokio::join!(
        update_instance(&client, &instances, &entry.folder, &options),
        configure_instance_version(&client, &instances, &entry.folder, "1.21.1"),
        rename_instance(&client, &instances, &entry.folder, "Renamed")
    );
    edited.expect("settings");
    prepared.expect("preparation");
    renamed.expect("rename");
    let profile = load_instance(&client, &instances.join(&entry.folder))
        .await
        .expect("profile");
    assert_eq!(profile.name, "Renamed");
    assert_eq!(profile.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(profile.max_memory_mb, options.max_memory_mb);
    assert_eq!(profile.jvm_arguments, options.jvm_arguments);
    assert_eq!(
        profile.installation.as_ref().expect("installation").state,
        licht_core::InstancePreparationState::Installing
    );
    service.shutdown().await.expect("shutdown");
    let service = database(&root).await;
    assert_eq!(
        load_instance(&service.client(), &instances.join(&entry.folder))
            .await
            .expect("persisted"),
        profile
    );
    service.shutdown().await.expect("shutdown");
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn rejected_database_writes_preserve_profiles_and_clean_incomplete_folders() {
    let root = scratch("sqlite-rejected-writes");
    let cache = SharedCache::at(&root);
    let service = database(&root).await;
    let client = service.client();
    let instances = root.join("instances");
    create_instance(&client, &instances, "World", "1.20.1", &plain_template())
        .await
        .expect("create");
    let original = load_instance(&client, &instances.join("World"))
        .await
        .expect("original");
    let mut invalid = settings();
    invalid.max_memory_mb = 0;
    assert!(matches!(
        update_instance(&client, &instances, "World", &invalid).await,
        Err(CoreError::InstanceMemory)
    ));
    assert_eq!(
        load_instance(&client, &instances.join("World"))
            .await
            .expect("invalid settings rejected"),
        original
    );
    let mut connection = open_db(&cache).await.expect("connection");
    sqlx::raw_sql("CREATE TRIGGER reject_instance_write BEFORE INSERT ON instances BEGIN SELECT RAISE(ABORT, 'rejected fixture write'); END;").execute(&mut connection).await.expect("reject writes");
    assert!(
        update_instance(&client, &instances, "World", &settings())
            .await
            .is_err()
    );
    assert!(
        rename_instance(&client, &instances, "World", "Changed")
            .await
            .is_err()
    );
    assert_eq!(
        load_instance(&client, &instances.join("World"))
            .await
            .expect("unchanged"),
        original
    );
    assert!(
        create_instance(&client, &instances, "Failed", "1.21.1", &plain_template())
            .await
            .is_err()
    );
    assert!(!instances.join("Failed").exists());
    assert!(
        create_pending_instance(
            &client,
            &instances,
            "Pending",
            InstanceSource::Vanilla {
                game_version: "1.21.1".into()
            },
            &plain_template()
        )
        .await
        .is_err()
    );
    assert!(!instances.join("Pending").exists());
    fs::write(
        instances.join("World").join("options.txt"),
        b"user settings",
    )
    .expect("game file");
    assert!(
        duplicate_instance(&client, &instances, "World", "Copy")
            .await
            .is_err()
    );
    assert!(!instances.join("Copy").exists());
    assert_eq!(
        fs::read(instances.join("World").join("options.txt")).expect("original file"),
        b"user settings"
    );
    connection.close().await.expect("close");
    service.shutdown().await.expect("shutdown");
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn deleting_a_profile_and_its_modpack_index_is_transactional() {
    let root = scratch("sqlite-delete-transaction");
    let service = database(&root).await;
    let client = service.client();
    let instances = root.join("instances");
    create_instance(&client, &instances, "World", "1.20.1", &plain_template())
        .await
        .expect("create");
    let mut connection = open_db(&SharedCache::at(&root)).await.expect("connection");
    sqlx::query("INSERT INTO modpack_indexes (instances_directory, instance_folder, name) VALUES (?1, 'World', 'Pack')")
        .bind(instances.to_str().expect("path")).execute(&mut connection).await.expect("modpack index");
    sqlx::raw_sql("CREATE TRIGGER reject_instance_delete BEFORE DELETE ON instances BEGIN SELECT RAISE(ABORT, 'rejected fixture deletion'); END;")
        .execute(&mut connection).await.expect("reject deletion");
    assert!(delete_instance(&client, &instances, "World").await.is_err());
    assert!(!instances.join("World").exists());
    let profiles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instances")
        .fetch_one(&mut connection)
        .await
        .expect("profile retained");
    let indexes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM modpack_indexes")
        .fetch_one(&mut connection)
        .await
        .expect("index retained");
    assert_eq!((profiles, indexes), (1, 1));
    sqlx::raw_sql("DROP TRIGGER reject_instance_delete")
        .execute(&mut connection)
        .await
        .expect("allow deletion");
    delete_instance(&client, &instances, "World")
        .await
        .expect("retry");
    let profiles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instances")
        .fetch_one(&mut connection)
        .await
        .expect("profiles");
    let indexes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM modpack_indexes")
        .fetch_one(&mut connection)
        .await
        .expect("indexes");
    assert_eq!((profiles, indexes), (0, 0));
    connection.close().await.expect("close");
    service.shutdown().await.expect("shutdown");
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn version_six_gains_an_empty_instances_table_and_preserves_existing_data() {
    let root = scratch("sqlite-schema-six");
    let cache = SharedCache::at(&root);
    let mut connection = open_db(&cache).await.expect("database");
    sqlx::raw_sql("DROP TABLE instances; INSERT INTO meta VALUES ('fixture', 'preserved'); PRAGMA user_version = 6;")
        .execute(&mut connection).await.expect("version six fixture");
    connection.close().await.expect("close");
    let old = root.join("instances").join("Legacy");
    write(&old, "instance.json", "invalid obsolete JSON");
    let service = database(&root).await;
    assert!(
        list_instances(&service.client(), &root.join("instances"))
            .await
            .expect("empty registry")
            .is_empty()
    );
    service.shutdown().await.expect("shutdown");
    let mut connection = open_db(&cache).await.expect("reopen");
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut connection)
        .await
        .expect("schema");
    assert_eq!(version, 7);
    let value: String = sqlx::query_scalar("SELECT value FROM meta WHERE key = 'fixture'")
        .fetch_one(&mut connection)
        .await
        .expect("metadata");
    assert_eq!(value, "preserved");
    assert_eq!(
        fs::read_to_string(old.join("instance.json")).expect("untouched"),
        "invalid obsolete JSON"
    );
    connection.close().await.expect("close");
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn corrupted_database_profiles_are_reported_instead_of_reset() {
    let root = scratch("sqlite-corrupted-profile");
    let service = database(&root).await;
    let client = service.client();
    let instances = root.join("instances");
    create_instance(&client, &instances, "World", "1.20.1", &plain_template())
        .await
        .expect("create");
    let mut connection = open_db(&SharedCache::at(&root)).await.expect("connection");
    sqlx::query("UPDATE instances SET jvm_arguments = 'not JSON'")
        .execute(&mut connection)
        .await
        .expect("corrupt arguments");
    assert!(matches!(
        load_instance(&client, &instances.join("World")).await,
        Err(CoreError::Instance(_))
    ));
    assert!(list_instances(&client, &instances).await.is_err());
    let stored: String = sqlx::query_scalar("SELECT jvm_arguments FROM instances")
        .fetch_one(&mut connection)
        .await
        .expect("retained data");
    assert_eq!(stored, "not JSON");
    sqlx::query("UPDATE instances SET jvm_arguments = '[]', version_id = '../escape'")
        .execute(&mut connection)
        .await
        .expect("unsafe version");
    assert!(matches!(
        load_instance(&client, &instances.join("World")).await,
        Err(CoreError::CachePath)
    ));
    connection.close().await.expect("close");
    service.shutdown().await.expect("shutdown");
    fs::remove_dir_all(root).expect("cleanup");
}
