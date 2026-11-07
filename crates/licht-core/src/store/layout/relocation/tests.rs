use std::fs;

use super::{DirectoryState, PendingMove, write_state};
use crate::SharedCache;
use crate::store::layout::progress::{StartupPhase, StartupReporter};
use crate::store::layout::{initialize_at, tests::fixture};

#[tokio::test]
async fn changing_directory_moves_content_after_restart_and_keeps_the_profile() {
    let (legacy, home) = fixture("relocation");
    initialize_at(&legacy, &home).await.expect("initialize");
    let source = crate::game_cache(&home).await.expect("source");
    fs::create_dir_all(source.root().join("runtime")).expect("runtime");
    let java = source.root().join("runtime/java");
    fs::write(&java, b"java").expect("java");
    let mut permissions = fs::metadata(&java).expect("Java metadata").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&java, permissions).expect("read-only runtime file");
    let instance = source.instance_dir("Survival").expect("instance");
    fs::create_dir_all(instance.join("saves")).expect("saves");
    fs::write(instance.join("saves/level.dat"), b"world").expect("world");
    fs::write(instance.join("saves/instance.json"), b"user content")
        .expect("unrelated JSON filename");
    fs::write(
        instance.join("instance.json"),
        serde_json::to_vec(&serde_json::json!({"javaPath": java})).expect("legacy profile"),
    )
    .expect("legacy profile file");
    fs::create_dir_all(source.cache_dir().expect("cache").join("icons")).expect("icons");
    fs::write(
        source.cache_dir().expect("cache").join("icons/one.png"),
        b"icon",
    )
    .expect("icon");
    let service = crate::DatabaseService::start(home.clone())
        .await
        .expect("service");
    let registered = crate::create_instance(
        &service.client(),
        &source.instances_dir().expect("instances"),
        "Registered",
        "1.21",
        &crate::global_launch(&crate::load_settings(&home).await.expect("settings")),
    )
    .await
    .expect("registered instance");
    let mut profile = crate::load_instance(
        &service.client(),
        &source.instance_dir(&registered.folder).expect("instance"),
    )
    .await
    .expect("profile");
    profile.java_path = Some(java.display().to_string());
    service
        .client()
        .write_instance_profile(
            &source.instance_dir(&registered.folder).expect("instance"),
            &profile,
        )
        .await
        .expect("Java override");
    let job = service
        .client()
        .create_pending_download_job(
            &source,
            "Survival",
            Some("1.21"),
            crate::DownloadJobOperation::Install,
        )
        .await
        .expect("job");
    service
        .client()
        .register_download_artifacts(&source, &job.id, std::slice::from_ref(&java))
        .await
        .expect("artifact");
    service.shutdown().await.expect("shutdown");
    let mut connection = super::super::db::open_db(&home).await.expect("database");
    sqlx::query("INSERT INTO native_generations VALUES (?1, '1.21', 'fingerprint', 'generation')")
        .bind(source.root().to_str().expect("source path"))
        .execute(&mut connection)
        .await
        .expect("native generation");
    sqlx::query(
        "INSERT INTO native_outputs VALUES (?1, '1.21', 'fingerprint', 'native.dll', 4, '0')",
    )
    .bind(source.root().to_str().expect("source path"))
    .execute(&mut connection)
    .await
    .expect("native output");
    let instances = source.instances_dir().expect("instances");
    sqlx::query("INSERT INTO modpack_indexes (instances_directory, instance_folder, name) VALUES (?1, ?2, 'Pack')")
        .bind(instances.to_str().expect("instances path")).bind(&registered.folder)
        .execute(&mut connection).await.expect("modpack index");
    sqlx::query("INSERT INTO modpack_dependencies VALUES (?1, ?2, 'minecraft', '1.21')")
        .bind(instances.to_str().expect("instances path"))
        .bind(&registered.folder)
        .execute(&mut connection)
        .await
        .expect("modpack dependency");
    use sqlx::Connection;
    connection.close().await.expect("close");
    let mut settings = crate::load_settings(&home).await.expect("settings");
    settings.java21 = Some(java.to_string_lossy().into_owned());
    settings.data_directory = Some(
        home.root()
            .parent()
            .expect("parent")
            .join("chosen")
            .display()
            .to_string(),
    );
    crate::save_settings(&home, &settings)
        .await
        .expect("save destination");
    assert!(java.exists(), "saving must not move live files");
    initialize_at(&legacy, &home)
        .await
        .expect("move on restart");
    let target = crate::game_cache(&home).await.expect("target");
    assert_eq!(
        fs::read(target.root().join("runtime/java")).expect("moved java"),
        b"java"
    );
    assert!(
        fs::metadata(target.root().join("runtime/java"))
            .expect("moved metadata")
            .permissions()
            .readonly()
    );
    assert_eq!(
        fs::read(
            target
                .instance_dir("Survival")
                .expect("instance")
                .join("saves/level.dat")
        )
        .expect("world"),
        b"world"
    );
    assert!(
        target
            .cache_dir()
            .expect("cache")
            .join("icons/one.png")
            .exists()
    );
    assert!(!java.exists());
    assert!(!instance.exists());
    let legacy_profile: serde_json::Value = serde_json::from_slice(
        &fs::read(
            target
                .instance_dir("Survival")
                .expect("instance")
                .join("instance.json"),
        )
        .expect("moved legacy profile"),
    )
    .expect("profile JSON");
    assert_eq!(
        legacy_profile["javaPath"],
        target.root().join("runtime/java").display().to_string()
    );
    assert!(home.root().join("settings.json").is_file());
    assert!(super::super::db::db_path(&home).is_file());
    assert_eq!(
        crate::load_settings(&home).await.expect("settings").java21,
        Some(target.root().join("runtime/java").display().to_string())
    );
    let service = crate::DatabaseService::start(home.clone())
        .await
        .expect("reopen");
    assert_eq!(
        service
            .client()
            .list_download_jobs(&target)
            .await
            .expect("jobs")
            .len(),
        1
    );
    let entries = crate::list_instances(
        &service.client(),
        &target.instances_dir().expect("instances"),
    )
    .await
    .expect("registered instances");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "Registered");
    assert_eq!(
        entries[0].java_path,
        Some(target.root().join("runtime/java").display().to_string())
    );
    service.shutdown().await.expect("shutdown");
    let mut connection = super::super::db::open_db(&home).await.expect("database");
    let artifact: String =
        sqlx::query_scalar("SELECT path FROM download_job_artifacts WHERE job_id = ?1")
            .bind(&job.id)
            .fetch_one(&mut connection)
            .await
            .expect("artifact path");
    assert_eq!(
        std::path::PathBuf::from(artifact),
        target.root().join("runtime/java")
    );
    let native_directory: String = sqlx::query_scalar("SELECT data_directory FROM native_outputs")
        .fetch_one(&mut connection)
        .await
        .expect("native scope");
    assert_eq!(std::path::PathBuf::from(native_directory), target.root());
    let modpack_directory: String =
        sqlx::query_scalar("SELECT instances_directory FROM modpack_dependencies")
            .fetch_one(&mut connection)
            .await
            .expect("modpack scope");
    assert_eq!(
        std::path::PathBuf::from(modpack_directory),
        target.instances_dir().expect("instances")
    );
    let foreign_key_errors = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut connection)
        .await
        .expect("foreign keys");
    assert!(foreign_key_errors.is_empty());
    connection.close().await.expect("close");
    initialize_at(&legacy, &home)
        .await
        .expect("idempotent restart");
}

async fn stored_state(home: &SharedCache) -> Option<serde_json::Value> {
    let mut connection = super::super::db::open_db(home).await.expect("database");
    let stored = super::super::db::get_meta(&mut connection, super::STATE_KEY)
        .await
        .expect("stored state");
    use sqlx::Connection;
    connection.close().await.expect("close");
    stored.map(|json| serde_json::from_str(&json).expect("state JSON"))
}

#[tokio::test]
async fn content_directory_state_lives_in_the_database() {
    let (legacy, home) = fixture("relocation-state");
    initialize_at(&legacy, &home).await.expect("initialize");
    let game = crate::game_cache(&home).await.expect("game");
    let state = stored_state(&home).await.expect("state in the database");
    assert_eq!(state["active"], game.root().display().to_string());
    assert!(state["pending"].is_null());
    assert!(!home.root().join("content-directory.json").exists());
}

async fn replace_state_with_legacy_file(home: &SharedCache, state: serde_json::Value) {
    let mut connection = super::super::db::open_db(home).await.expect("database");
    super::super::db::clear_meta(&mut connection, super::STATE_KEY)
        .await
        .expect("clear state");
    use sqlx::Connection;
    connection.close().await.expect("close");
    fs::write(
        home.root().join("content-directory.json"),
        serde_json::to_vec(&state).expect("legacy state"),
    )
    .expect("legacy state file");
}

#[tokio::test]
async fn legacy_state_file_is_imported_once_and_removed() {
    let (legacy, home) = fixture("relocation-state-import");
    initialize_at(&legacy, &home).await.expect("initialize");
    let game = crate::game_cache(&home).await.expect("game");
    let file = home.root().join("content-directory.json");
    replace_state_with_legacy_file(
        &home,
        serde_json::json!({"active": game.root(), "pending": null}),
    )
    .await;
    initialize_at(&legacy, &home).await.expect("import");
    assert!(!file.exists());
    let state = stored_state(&home).await.expect("imported state");
    assert_eq!(state["active"], game.root().display().to_string());

    // A leftover file must not override the state already in the database.
    let elsewhere = home.root().parent().expect("parent").join("elsewhere/data");
    fs::write(
        &file,
        serde_json::to_vec(&serde_json::json!({"active": elsewhere, "pending": null}))
            .expect("stale state"),
    )
    .expect("stale state file");
    initialize_at(&legacy, &home)
        .await
        .expect("ignore stale file");
    assert!(!file.exists());
    let state = stored_state(&home).await.expect("database state");
    assert_eq!(state["active"], game.root().display().to_string());
}

#[tokio::test]
async fn a_move_pending_in_the_legacy_file_finishes_after_import() {
    for copied in [false, true] {
        let (legacy, home) = fixture(&format!("relocation-state-pending-{copied}"));
        initialize_at(&legacy, &home).await.expect("initialize");
        let source = crate::game_cache(&home).await.expect("source");
        fs::write(source.root().join("file"), b"original").expect("file");
        let mut settings = crate::load_settings(&home).await.expect("settings");
        settings.data_directory = Some(
            home.root()
                .parent()
                .expect("parent")
                .join("chosen")
                .display()
                .to_string(),
        );
        crate::save_settings(&home, &settings)
            .await
            .expect("save destination");
        let target = crate::game_cache(&home).await.expect("target");
        if copied {
            // The copy finished and cleanup already removed the source; only the
            // imported `copied` flag lets startup finish instead of failing.
            fs::create_dir_all(target.root()).expect("target");
            fs::write(target.root().join("file"), b"original").expect("copied file");
            fs::remove_dir_all(source.root()).expect("cleaned source");
        }
        replace_state_with_legacy_file(
            &home,
            serde_json::json!({
                "active": source.root(),
                "pending": {"destination": target.root(), "copied": copied},
            }),
        )
        .await;
        initialize_at(&legacy, &home).await.expect("finish move");
        assert_eq!(
            fs::read(target.root().join("file")).expect("moved file"),
            b"original"
        );
        assert!(!source.root().exists());
        assert!(!home.root().join("content-directory.json").exists());
        let state = stored_state(&home).await.expect("final state");
        assert_eq!(state["active"], target.root().display().to_string());
        assert!(state["pending"].is_null());
    }
}

#[tokio::test]
async fn interrupted_copy_resumes_without_overwriting_different_files() {
    let (legacy, home) = fixture("relocation-interrupted");
    initialize_at(&legacy, &home).await.expect("initialize");
    let source = crate::game_cache(&home).await.expect("source");
    fs::write(source.root().join("file"), b"original").expect("file");
    let mut settings = crate::load_settings(&home).await.expect("settings");
    settings.data_directory = Some(
        home.root()
            .parent()
            .expect("parent")
            .join("chosen")
            .display()
            .to_string(),
    );
    crate::save_settings(&home, &settings).await.expect("save");
    let interrupted = StartupReporter::new(|progress| {
        if progress.phase == StartupPhase::MigratingDatabase {
            return Err(std::io::Error::other("simulated shutdown after copying").into());
        }
        Ok(())
    });
    assert!(super::apply(&home, &interrupted).await.is_err());
    let target = crate::game_cache(&home).await.expect("target");
    assert!(source.root().join("file").exists());
    assert!(target.root().join("file").exists());
    initialize_at(&legacy, &home).await.expect("resume");
    assert_eq!(
        fs::read(target.root().join("file")).expect("target"),
        b"original"
    );
    assert!(!source.root().exists());
}

#[tokio::test]
async fn interrupted_cleanup_preserves_changed_source_and_resumes_after_resolution() {
    let (legacy, home) = fixture("relocation-cleanup");
    initialize_at(&legacy, &home).await.expect("initialize");
    let source = crate::game_cache(&home).await.expect("source");
    fs::write(source.root().join("file"), b"original").expect("source");
    let mut settings = crate::load_settings(&home).await.expect("settings");
    settings.data_directory = Some(
        home.root()
            .parent()
            .expect("parent")
            .join("chosen")
            .display()
            .to_string(),
    );
    crate::save_settings(&home, &settings).await.expect("save");
    let target = crate::game_cache(&home).await.expect("target");
    fs::create_dir_all(target.root()).expect("target");
    fs::write(target.root().join("file"), b"older copy").expect("interrupted copy");
    write_state(
        &home,
        &DirectoryState {
            active: source.root().to_path_buf(),
            pending: Some(PendingMove {
                destination: target.root().to_path_buf(),
                copied: true,
            }),
        },
    )
    .await
    .expect("copied checkpoint");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert_eq!(
        fs::read(source.root().join("file")).expect("preserved"),
        b"original"
    );
    fs::write(target.root().join("file"), b"original").expect("resolve");
    // A prior cleanup may already have removed an entire source subtree.
    fs::remove_dir(source.instances_dir().expect("instances")).expect("partial cleanup");
    initialize_at(&legacy, &home).await.expect("resume cleanup");
    assert!(!source.root().exists());
}

#[tokio::test]
async fn rejects_overlapping_roots_and_unavailable_previous_directories() {
    for (name, unavailable) in [("nested", false), ("missing", true)] {
        let (legacy, home) = fixture(name);
        initialize_at(&legacy, &home).await.expect("initialize");
        let source = crate::game_cache(&home).await.expect("source");
        let mut settings = crate::load_settings(&home).await.expect("settings");
        let parent = if unavailable {
            fs::remove_dir(source.root()).expect("simulate disconnected source");
            home.root().parent().expect("parent").join("chosen")
        } else {
            source.root().join("nested")
        };
        settings.data_directory = Some(parent.display().to_string());
        crate::save_settings(&home, &settings).await.expect("save");
        assert!(initialize_at(&legacy, &home).await.is_err());
        assert!(home.root().join("settings.json").is_file());
        if !unavailable {
            assert!(!source.root().join("nested").exists());
            settings.data_directory = None;
            crate::save_settings(&home, &settings)
                .await
                .expect("restore previous choice");
            initialize_at(&legacy, &home)
                .await
                .expect("cancel invalid move");
        }
    }
}

#[tokio::test]
async fn isolated_profile_does_not_import_legacy_settings_or_data() {
    let (legacy, default_home) = fixture("isolated");
    let mut settings = crate::load_settings(&legacy).await.expect("settings");
    settings.username = "LegacyPlayer".to_string();
    crate::save_settings(&legacy, &settings)
        .await
        .expect("legacy settings");
    let isolated = SharedCache::at(
        default_home
            .root()
            .parent()
            .expect("parent")
            .join("isolated"),
    );
    super::super::initialize_at_with_progress(None, &isolated, &StartupReporter::default())
        .await
        .expect("isolated startup");
    assert_eq!(
        crate::load_settings(&isolated)
            .await
            .expect("isolated settings")
            .username,
        "Steve"
    );
    assert!(!default_home.root().exists());
    assert_eq!(
        crate::load_settings(&legacy)
            .await
            .expect("legacy unchanged")
            .username,
        "LegacyPlayer"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_links_are_rejected_without_touching_external_files() {
    let (legacy, home) = fixture("relocation-link");
    initialize_at(&legacy, &home).await.expect("initialize");
    let source = crate::game_cache(&home).await.expect("source");
    let external = home.root().parent().expect("parent").join("external");
    fs::write(&external, b"external data").expect("external");
    std::os::unix::fs::symlink(&external, source.root().join("linked")).expect("link");
    let mut settings = crate::load_settings(&home).await.expect("settings");
    settings.data_directory = Some(
        home.root()
            .parent()
            .expect("parent")
            .join("chosen")
            .display()
            .to_string(),
    );
    crate::save_settings(&home, &settings).await.expect("save");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert_eq!(
        fs::read(external).expect("external preserved"),
        b"external data"
    );
    assert!(
        fs::symlink_metadata(source.root().join("linked"))
            .expect("source link")
            .file_type()
            .is_symlink()
    );
}

#[tokio::test]
async fn conflicting_destination_preserves_files_and_can_be_retried() {
    let (legacy, home) = fixture("relocation-conflict");
    initialize_at(&legacy, &home).await.expect("initialize");
    let source = crate::game_cache(&home).await.expect("source");
    fs::write(source.root().join("file"), b"original").expect("source");
    let mut settings = crate::load_settings(&home).await.expect("settings");
    settings.data_directory = Some(
        home.root()
            .parent()
            .expect("parent")
            .join("chosen")
            .display()
            .to_string(),
    );
    crate::save_settings(&home, &settings).await.expect("save");
    let target = crate::game_cache(&home).await.expect("target");
    fs::create_dir_all(target.root()).expect("target");
    fs::write(target.root().join("file"), b"different").expect("conflict");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert_eq!(
        fs::read(source.root().join("file")).expect("source preserved"),
        b"original"
    );
    assert_eq!(
        fs::read(target.root().join("file")).expect("target preserved"),
        b"different"
    );
    fs::remove_file(target.root().join("file")).expect("resolve conflict");
    initialize_at(&legacy, &home).await.expect("retry");
    assert_eq!(
        fs::read(target.root().join("file")).expect("copied"),
        b"original"
    );
    assert!(!source.root().join("file").exists());
    settings.data_directory = None;
    crate::save_settings(&home, &settings)
        .await
        .expect("choose default again");
    initialize_at(&legacy, &home).await.expect("move back");
    assert_eq!(
        fs::read(source.root().join("file")).expect("moved back"),
        b"original"
    );
}
