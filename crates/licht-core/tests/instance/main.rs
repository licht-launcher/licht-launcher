use std::fs;
use std::path::{Path, PathBuf};

use licht_core::{
    CoreError, GlobalLaunch, InstanceSettings, create_instance, delete_instance,
    duplicate_instance, instance_launch, list_instances, load_instance, parse_instance,
    rename_instance, resolved_java_path, resolved_launch, touch_last_opened, update_instance,
    update_instance_version,
};

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("licht-instance-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("scratch directory");
    root
}

fn plain_template() -> GlobalLaunch {
    GlobalLaunch {
        max_memory_mb: 2048,
        jvm_arguments: Vec::new(),
        fullscreen: false,
        width: None,
        height: None,
    }
}

fn write(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).expect("directory");
    fs::write(dir.join(name), body).expect("file");
}

#[tokio::test]
async fn unregistered_folders_are_not_initialized_or_imported() {
    let root = scratch("migrate");
    let dir = root.join("instances").join("1.20.1");
    fs::create_dir_all(&dir).expect("instance folder");

    let service = database(&root).await;
    let client = service.client();
    assert!(
        matches!(load_instance(&client, &dir).await, Err(CoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound)
    );
    assert!(!dir.join("instance.json").exists());
    let body = r#"{"schema":1,"name":"Legacy","versionId":"1.20.1","minMemoryMb":512,"maxMemoryMb":2048,"jvmArguments":[]}"#;
    fs::write(dir.join("instance.json"), body).expect("legacy profile");
    assert!(load_instance(&client, &dir).await.is_err());
    assert!(
        list_instances(&client, &root.join("instances"))
            .await
            .expect("list")
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(dir.join("instance.json")).expect("untouched"),
        body
    );
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_one_is_read_back_unchanged() {
    let root = scratch("read");
    let dir = root.join("Survival");
    write(
        &dir,
        "instance.json",
        r#"{
  "schema": 1,
  "name": "Survival",
  "versionId": "1.20.1",
  "minMemoryMb": 1024,
  "maxMemoryMb": 4096,
  "jvmArguments": ["-XX:+UseG1GC"],
  "width": 1280,
  "height": 720
}"#,
    );
    let before = fs::read_to_string(dir.join("instance.json")).expect("original");
    let instance = parse_instance(&before).expect("read");
    assert_eq!(instance.name, "Survival");
    assert_eq!(instance.version_id.as_deref(), Some("1.20.1"));
    assert_eq!(instance.min_memory_mb, 1024);
    assert_eq!(instance.max_memory_mb, 4096);
    assert_eq!(instance.jvm_arguments, vec!["-XX:+UseG1GC".to_string()]);
    assert!(!instance.fullscreen);
    assert_eq!(instance.width, Some(1280));
    assert_eq!(instance.height, Some(720));
    assert_eq!(
        fs::read_to_string(dir.join("instance.json")).expect("untouched"),
        before
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_broken_file_and_a_newer_schema_are_left_in_place() {
    let root = scratch("reject");
    let broken = root.join("broken");
    write(&broken, "instance.json", "{ not json");
    let broken_body = fs::read_to_string(broken.join("instance.json")).expect("broken");
    assert!(matches!(
        parse_instance(&broken_body),
        Err(CoreError::Instance(_))
    ));
    assert_eq!(
        fs::read_to_string(broken.join("instance.json")).expect("still broken"),
        broken_body
    );

    let future = root.join("future");
    write(
        &future,
        "instance.json",
        r#"{"schema": 3, "name": "Later"}"#,
    );
    let future_body = fs::read_to_string(future.join("instance.json")).expect("future");
    assert!(matches!(
        parse_instance(&future_body),
        Err(CoreError::InstanceSchema { schema: 3 })
    ));
    assert_eq!(
        fs::read_to_string(future.join("instance.json")).expect("still future"),
        future_body
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn memory_must_be_a_positive_range() {
    let json = r#"{
  "schema": 1,
  "name": "1.20.1",
  "versionId": "1.20.1",
  "minMemoryMb": 2048,
  "maxMemoryMb": 512,
  "jvmArguments": [],
  "width": null,
  "height": null
}"#;
    assert!(matches!(
        parse_instance(json),
        Err(CoreError::InstanceMemory)
    ));
}

#[tokio::test]
async fn a_minecraft_child_is_not_its_own_instance() {
    let root = scratch("list");
    let instances = root.join("instances");
    let legacy = instances.join("1.5.2");
    let service = database(&root).await;
    let client = service.client();
    create_instance(&client, &instances, "1.5.2", "1.5.2", &plain_template())
        .await
        .expect("legacy");
    create_instance(&client, &instances, "1.20.1", "1.20.1", &plain_template())
        .await
        .expect("modern");
    fs::create_dir_all(legacy.join(".minecraft").join("saves")).expect("legacy game dir");

    let listed = list_instances(&client, &instances).await.expect("list");
    let ids: Vec<&str> = listed
        .iter()
        .map(|instance| instance.folder.as_str())
        .collect();
    assert_eq!(ids, vec!["1.20.1", "1.5.2"]);
    assert!(!legacy.join("instance.json").exists());
    assert!(!legacy.join(".minecraft").join("instance.json").exists());
    assert!(
        list_instances(&client, &root.join("missing"))
            .await
            .expect("absent")
            .is_empty()
    );
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn create_rename_duplicate_and_delete_keep_the_folder_stable() {
    let root = scratch("crud");
    let instances = root.join("instances");
    let service = database(&root).await;
    let client = service.client();

    let created = create_instance(
        &client,
        &instances,
        " My World ",
        "1.20.1",
        &plain_template(),
    )
    .await
    .expect("create");
    assert_eq!(created.folder, "My-World");
    assert_eq!(created.name, "My World");
    assert_eq!(created.version_id.as_deref(), Some("1.20.1"));
    assert!(created.icon_path.is_none());
    assert!(!instances.join("My-World").join("instance.json").exists());
    fs::write(instances.join("My-World").join("icon.png"), [0u8, 1, 2, 3]).expect("icon");
    let with_icon = list_instances(&client, &instances)
        .await
        .expect("list with icon");
    let path = with_icon
        .iter()
        .find(|entry| entry.folder == "My-World")
        .expect("instance")
        .icon_path
        .as_deref()
        .expect("icon path");
    assert!(path.ends_with("icon.png"));
    assert!(std::path::Path::new(path).is_file());
    assert!(matches!(
        create_instance(&client, &instances, "My World", "1.20.1", &plain_template()).await,
        Err(CoreError::InstanceExists)
    ));
    assert!(matches!(
        create_instance(&client, &instances, "   ", "1.20.1", &plain_template()).await,
        Err(CoreError::InstanceName)
    ));

    let renamed = rename_instance(&client, &instances, "My-World", "Survival")
        .await
        .expect("rename");
    assert_eq!(renamed.folder, "My-World");
    assert_eq!(renamed.name, "Survival");
    assert!(instances.join("My-World").is_dir());
    assert!(!instances.join("Survival").exists());

    fs::create_dir_all(instances.join("My-World").join(".minecraft").join("saves"))
        .expect("saves dir");
    fs::write(
        instances
            .join("My-World")
            .join(".minecraft")
            .join("saves")
            .join("level.dat"),
        "world",
    )
    .expect("save");

    let copy = duplicate_instance(&service.client(), &instances, "My-World", "Survival copy")
        .await
        .expect("duplicate");
    assert_eq!(copy.folder, "Survival-copy");
    assert_eq!(copy.name, "Survival copy");
    assert_eq!(copy.version_id.as_deref(), Some("1.20.1"));
    assert_eq!(
        fs::read_to_string(
            instances
                .join("Survival-copy")
                .join(".minecraft")
                .join("saves")
                .join("level.dat")
        )
        .expect("copied save"),
        "world"
    );
    assert!(instances.join("My-World").is_dir());

    let stored = load_instance(&client, &instances.join("My-World"))
        .await
        .expect("stored");
    assert_eq!(instance_launch(&stored), plain_template());
    assert_eq!(stored.min_memory_mb, 512);

    delete_instance(&service.client(), &instances, "Survival-copy")
        .await
        .expect("delete");
    assert!(!instances.join("Survival-copy").exists());
    assert!(instances.join("My-World").is_dir());
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn a_new_instance_copies_the_template_and_leaves_an_older_one_alone() {
    let root = scratch("template");
    let instances = root.join("instances");
    let service = database(&root).await;
    let template = GlobalLaunch {
        max_memory_mb: 4096,
        jvm_arguments: vec!["-XX:+UseG1GC".to_string()],
        fullscreen: true,
        width: Some(854),
        height: Some(480),
    };
    let created = create_instance(
        &service.client(),
        &instances,
        "New World",
        "1.21.1",
        &template,
    )
    .await
    .expect("create");
    assert!(created.fullscreen);
    assert_eq!(created.max_memory_mb, 4096);
    assert_eq!(created.jvm_arguments, vec!["-XX:+UseG1GC".to_string()]);
    assert_eq!(created.width, Some(854));
    assert_eq!(created.height, Some(480));
    assert_eq!(created.min_memory_mb, 512);

    let older = instances.join("Older");
    write(
        &older,
        "instance.json",
        r#"{
  "schema": 1,
  "name": "Older",
  "versionId": "1.20.1",
  "minMemoryMb": 512,
  "maxMemoryMb": 2048,
  "jvmArguments": [],
  "width": 1280,
  "height": 720
}"#,
    );
    let before = fs::read_to_string(older.join("instance.json")).expect("original");
    assert!(load_instance(&service.client(), &older).await.is_err());
    let loaded = parse_instance(&before).expect("older");
    assert!(!loaded.fullscreen);
    assert_eq!(loaded.width, Some(1280));
    assert_eq!(
        fs::read_to_string(older.join("instance.json")).expect("untouched"),
        before
    );
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn older_files_default_overrides_to_off() {
    let root = scratch("overrides-default");
    let dir = root.join("Survival");
    write(
        &dir,
        "instance.json",
        r#"{
  "schema": 1,
  "name": "Survival",
  "versionId": "1.20.1",
  "minMemoryMb": 512,
  "maxMemoryMb": 4096,
  "jvmArguments": ["-XX:+UseG1GC"],
  "fullscreen": true,
  "width": 1280,
  "height": 720
}"#,
    );
    let instance = parse_instance(&fs::read_to_string(dir.join("instance.json")).expect("json"))
        .expect("read");
    assert!(!instance.override_window);
    assert!(!instance.override_memory);
    assert!(!instance.override_java);
    assert!(!instance.override_jvm_arguments);
    assert_eq!(instance.java_path, None);
    assert!(instance.fullscreen);
    assert_eq!(instance.max_memory_mb, 4096);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolved_launch_uses_global_until_overrides_turn_on() {
    let instance = parse_instance(&{
        let root = scratch("resolve");
        let dir = root.join("Game");
        write(
            &dir,
            "instance.json",
            r#"{
  "schema": 1,
  "name": "Game",
  "versionId": "1.20.1",
  "minMemoryMb": 512,
  "maxMemoryMb": 8192,
  "jvmArguments": ["-Xss1M"],
  "fullscreen": true,
  "width": 1920,
  "height": 1080,
  "overrideWindow": false,
  "overrideMemory": false,
  "overrideJava": false,
  "overrideJvmArguments": false,
  "javaPath": "C:/custom/java.exe"
}"#,
        );
        fs::read_to_string(dir.join("instance.json")).expect("json")
    })
    .expect("instance");
    let global = GlobalLaunch {
        max_memory_mb: 2048,
        jvm_arguments: vec!["-XX:+UseG1GC".to_string()],
        fullscreen: false,
        width: Some(854),
        height: Some(480),
    };
    assert_eq!(resolved_launch(&instance, &global), global);
    assert_eq!(resolved_java_path(&instance), None);

    let mut customized = instance.clone();
    customized.override_window = true;
    customized.override_memory = true;
    customized.override_jvm_arguments = true;
    customized.override_java = true;
    let merged = resolved_launch(&customized, &global);
    assert_eq!(merged.max_memory_mb, 8192);
    assert_eq!(merged.jvm_arguments, vec!["-Xss1M".to_string()]);
    assert!(merged.fullscreen);
    assert_eq!(merged.width, Some(1920));
    assert_eq!(merged.height, Some(1080));
    assert_eq!(
        resolved_java_path(&customized),
        Some(std::path::PathBuf::from("C:/custom/java.exe"))
    );
}

#[tokio::test]
async fn update_instance_persists_overrides_and_keeps_name() {
    let root = scratch("update");
    let instances = root.join("instances");
    let service = database(&root).await;
    let client = service.client();
    let created = create_instance(&client, &instances, "My World", "1.20.1", &plain_template())
        .await
        .expect("create");
    let updated = update_instance(
        &client,
        &instances,
        &created.folder,
        &InstanceSettings {
            override_window: true,
            override_memory: true,
            override_java: true,
            override_jvm_arguments: true,
            java_path: Some("  C:/jdk/bin/java.exe  ".to_string()),
            max_memory_mb: 6144,
            jvm_arguments: vec!["-XX:+UseZGC".to_string()],
            fullscreen: true,
            width: Some(1600),
            height: Some(900),
        },
    )
    .await
    .expect("update");
    assert_eq!(updated.folder, "My-World");
    assert_eq!(updated.name, "My World");
    assert_eq!(updated.version_id.as_deref(), Some("1.20.1"));
    assert!(updated.override_window);
    assert!(updated.override_memory);
    assert!(updated.override_java);
    assert!(updated.override_jvm_arguments);
    assert_eq!(updated.java_path.as_deref(), Some("C:/jdk/bin/java.exe"));
    assert_eq!(updated.max_memory_mb, 6144);
    assert_eq!(updated.jvm_arguments, vec!["-XX:+UseZGC".to_string()]);
    assert!(updated.fullscreen);
    assert_eq!(updated.width, Some(1600));
    assert_eq!(updated.height, Some(900));
    service.shutdown().await.expect("shutdown");
    let service = database(&root).await;
    assert_eq!(
        list_instances(&service.client(), &instances)
            .await
            .expect("reopened"),
        vec![updated]
    );
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn update_instance_version_changes_only_the_game_version() {
    let root = scratch("version");
    let instances = root.join("instances");
    let service = database(&root).await;
    let client = service.client();
    let created = create_instance(&client, &instances, "My World", "1.20.1", &plain_template())
        .await
        .expect("create");
    let updated = update_instance_version(&client, &instances, &created.folder, "1.21.1")
        .await
        .expect("version");
    assert_eq!(updated.folder, "My-World");
    assert_eq!(updated.name, "My World");
    assert_eq!(updated.version_id.as_deref(), Some("1.21.1"));
    assert_eq!(updated.max_memory_mb, created.max_memory_mb);
    service.shutdown().await.expect("shutdown");
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn touch_last_opened_writes_rfc3339_and_duplicate_clears_it() {
    let root = scratch("opened");
    let instances = root.join("instances");
    let service = database(&root).await;
    let client = service.client();
    let created = create_instance(&client, &instances, "Classic", "1.5.2", &plain_template())
        .await
        .expect("create");
    assert_eq!(created.last_opened_at, None);

    let touched = touch_last_opened(&client, &instances, &created.folder)
        .await
        .expect("touch");
    let stamp = touched.last_opened_at.expect("timestamp");
    assert!(
        stamp.ends_with('Z') && stamp.contains('T') && stamp.len() == 20,
        "unexpected stamp {stamp}"
    );
    let again = load_instance(&client, &instances.join(&created.folder))
        .await
        .expect("reload");
    assert_eq!(again.last_opened_at.as_deref(), Some(stamp.as_str()));

    let copy = duplicate_instance(
        &service.client(),
        &instances,
        &created.folder,
        "Classic Copy",
    )
    .await
    .expect("dup");
    service.shutdown().await.expect("shutdown");
    assert_eq!(copy.last_opened_at, None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn missing_last_opened_at_defaults_to_none() {
    let root = scratch("legacy-opened");
    let dir = root.join("Legacy");
    write(
        &dir,
        "instance.json",
        r#"{
  "schema": 1,
  "name": "Legacy",
  "versionId": "1.20.1",
  "minMemoryMb": 512,
  "maxMemoryMb": 2048,
  "jvmArguments": []
}"#,
    );
    let instance = parse_instance(&fs::read_to_string(dir.join("instance.json")).expect("json"))
        .expect("load");
    assert_eq!(instance.last_opened_at, None);
    let _ = fs::remove_dir_all(root);
}
async fn database(root: &Path) -> licht_core::DatabaseService {
    licht_core::DatabaseService::start(licht_core::SharedCache::at(root))
        .await
        .expect("database")
}

mod preparation;
mod sqlite;
