use std::fs;
use std::path::PathBuf;

use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};

use super::*;

pub(super) fn fixture(name: &str) -> (SharedCache, SharedCache) {
    let root = std::env::temp_dir().join(format!(
        "licht-layout-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    (
        SharedCache::at(
            root.join("legacy")
                .join("Licht")
                .join("Licht Launcher")
                .join("data"),
        ),
        SharedCache::at(root.join("Licht Launcher")),
    )
}

async fn legacy_database(legacy: &SharedCache) -> SqliteConnection {
    let connection = db::open_migrated(legacy).await.expect("create schema");
    connection.close().await.expect("close current filename");
    fs::rename(db::db_path(legacy), legacy.root().join("licht.sqlite")).expect("legacy filename");
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(legacy.root().join("licht.sqlite"))
            .journal_mode(SqliteJournalMode::Wal),
    )
    .await
    .expect("legacy database")
}

#[tokio::test]
async fn fresh_layout_places_settings_and_wal_database_at_the_launcher_root() {
    let (legacy, home) = fixture("fresh");
    initialize_at(&legacy, &home).await.expect("initialize");
    assert_eq!(
        crate::settings::testing::game_cache_at(&home)
            .await
            .expect("game")
            .root(),
        home.root().join("data")
    );
    for directory in ["cache", "data", "instances"] {
        assert!(home.root().join(directory).is_dir());
    }
    assert!(home.root().join("settings.json").is_file());
    let mut connection = db::open_migrated(&home).await.expect("database");
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut connection)
        .await
        .expect("mode");
    assert_eq!(mode, "wal");
    assert_eq!(db::db_path(&home), home.root().join("licht.db"));
    sqlx::query("INSERT INTO meta VALUES ('wal-test', 'value')")
        .execute(&mut connection)
        .await
        .expect("write");
    assert!(home.root().join("licht.db-wal").is_file());
    assert!(home.root().join("licht.db-shm").is_file());
    connection.close().await.expect("close");
}

#[tokio::test]
async fn migration_preserves_files_settings_committed_wal_and_job_paths() {
    let (legacy, home) = fixture("preserve");
    fs::create_dir_all(legacy.root().join("libraries")).expect("library directory");
    let artifact = legacy.root().join("libraries").join("one.jar");
    fs::write(&artifact, b"library").expect("library");
    fs::write(artifact.with_extension("jar.part"), b"partial").expect("partial");
    let mut settings = crate::settings::load_settings_at(&legacy)
        .await
        .expect("settings");
    settings.download_concurrency = 3;
    settings.java21 = Some(artifact.to_string_lossy().into_owned());
    // Legacy profiles kept every setting in settings.json.
    fs::write(
        legacy.root().join("settings.json"),
        serde_json::to_vec(&settings).expect("settings JSON"),
    )
    .expect("legacy settings");
    let instance = legacy.instance_dir("Survival").expect("instance");
    fs::create_dir_all(instance.join("saves")).expect("save directory");
    fs::write(instance.join("saves").join("level.dat"), b"world").expect("world");
    fs::write(
        instance.join("instance.json"),
        serde_json::to_vec(&serde_json::json!({"javaPath": artifact})).expect("profile"),
    )
    .expect("profile file");
    let service =
        crate::DatabaseService::start(crate::Database::open(&legacy).await.expect("database"))
            .await
            .expect("legacy service");
    let job = service
        .client()
        .create_pending_download_job(
            &legacy,
            "Survival",
            Some("1.21"),
            crate::DownloadJobOperation::Install,
        )
        .await
        .expect("job");
    service
        .client()
        .register_download_artifacts(&legacy, &job.id, std::slice::from_ref(&artifact))
        .await
        .expect("artifact");
    service.shutdown().await.expect("close legacy service");
    let mut connection = legacy_database(&legacy).await;
    sqlx::query("INSERT INTO meta VALUES ('microsoft_account_uuids', 'fixture-uuid')")
        .execute(&mut connection)
        .await
        .expect("committed WAL metadata");

    initialize_at(&legacy, &home).await.expect("migrate");
    let new_game = crate::settings::testing::game_cache_at(&home)
        .await
        .expect("new game");
    assert_eq!(
        fs::read(new_game.root().join("libraries/one.jar")).expect("migrated library"),
        b"library"
    );
    assert_eq!(
        fs::read(
            new_game
                .instance_dir("Survival")
                .expect("instance")
                .join("saves/level.dat")
        )
        .expect("migrated world"),
        b"world"
    );
    assert!(!artifact.exists(), "the verified legacy copy is removed");
    let migrated = crate::settings::load_settings_at(&home)
        .await
        .expect("migrated settings");
    assert_eq!(migrated.download_concurrency, 3);
    assert_eq!(
        migrated.java21,
        Some(
            new_game
                .root()
                .join("libraries")
                .join("one.jar")
                .to_string_lossy()
                .into_owned()
        )
    );
    let profile: serde_json::Value = serde_json::from_slice(
        &fs::read(
            new_game
                .instance_dir("Survival")
                .expect("instance")
                .join("instance.json"),
        )
        .expect("profile"),
    )
    .expect("JSON");
    assert_eq!(
        profile["javaPath"],
        new_game
            .root()
            .join("libraries")
            .join("one.jar")
            .to_string_lossy()
            .as_ref()
    );
    let mut migrated = db::open_migrated(&home).await.expect("migrated database");
    let remembered: String =
        sqlx::query_scalar("SELECT value FROM meta WHERE key = 'microsoft_account_uuids'")
            .fetch_one(&mut migrated)
            .await
            .expect("WAL value");
    assert_eq!(remembered, "fixture-uuid");
    let directory: String =
        sqlx::query_scalar("SELECT data_directory FROM download_jobs WHERE id = ?1")
            .bind(&job.id)
            .fetch_one(&mut migrated)
            .await
            .expect("scope");
    assert_eq!(PathBuf::from(directory), new_game.root());
    let path: String =
        sqlx::query_scalar("SELECT path FROM download_job_artifacts WHERE job_id = ?1")
            .bind(&job.id)
            .fetch_one(&mut migrated)
            .await
            .expect("artifact path");
    assert_eq!(
        PathBuf::from(path),
        new_game.root().join("libraries").join("one.jar")
    );
    migrated.close().await.expect("close migrated");
    initialize_at(&legacy, &home)
        .await
        .expect("repeat initialization");
    connection.close().await.expect("close legacy");
}

#[tokio::test]
async fn conflicting_destination_preserves_source_and_retries_after_resolution() {
    let (legacy, home) = fixture("conflict");
    fs::create_dir_all(legacy.root().join("versions")).expect("legacy versions");
    fs::write(legacy.root().join("versions/file.json"), b"original").expect("original");
    fs::create_dir_all(home.root().join("data/versions")).expect("destination");
    let conflict = home.root().join("data/versions/file.json");
    fs::write(&conflict, b"different").expect("conflict");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert!(!db::db_path(&home).exists());
    assert_eq!(
        fs::read(&conflict).expect("preserved destination"),
        b"different"
    );
    assert_eq!(
        fs::read(legacy.root().join("versions/file.json")).expect("preserved source"),
        b"original"
    );
    fs::remove_file(conflict).expect("resolve fixture conflict");
    initialize_at(&legacy, &home).await.expect("retry");
    assert_eq!(
        fs::read(home.root().join("data/versions/file.json")).expect("copied"),
        b"original"
    );
}

#[tokio::test]
async fn migration_preserves_custom_game_directory_and_remaps_its_instance_root() {
    let (legacy, home) = fixture("custom");
    let chosen = home.root().parent().expect("parent").join("custom");
    fs::create_dir_all(legacy.root()).expect("legacy");
    let mut settings = crate::settings::load_settings_at(&legacy)
        .await
        .expect("defaults");
    settings.data_directory = Some(chosen.to_string_lossy().into_owned());
    crate::settings::testing::save_settings_at(&legacy, &settings)
        .await
        .expect("settings");
    let old_game = SharedCache::at(chosen.join("Licht Launcher"));
    fs::create_dir_all(old_game.root().join("versions")).expect("versions");
    fs::write(old_game.root().join("versions/custom.json"), b"version").expect("version");
    let old_instance = old_game.instance_dir("Custom").expect("old instance");
    fs::create_dir_all(&old_instance).expect("instance");
    fs::write(old_instance.join("save.dat"), b"save").expect("save");
    initialize_at(&legacy, &home)
        .await
        .expect("custom migration");
    let game = crate::settings::testing::game_cache_at(&home)
        .await
        .expect("custom game");
    assert_eq!(game.root(), chosen.join("Licht Launcher/data"));
    assert_eq!(
        fs::read(game.root().join("versions/custom.json")).expect("version"),
        b"version"
    );
    assert_eq!(
        fs::read(
            game.instance_dir("Custom")
                .expect("new instance")
                .join("save.dat")
        )
        .expect("save"),
        b"save"
    );
    assert!(
        !old_instance.exists(),
        "the verified old instance is removed"
    );
    assert!(!old_game.root().join("versions").exists());
}

#[tokio::test]
async fn migration_retries_after_a_late_settings_conflict_without_rewriting_original_profiles() {
    let (legacy, home) = fixture("late-conflict");
    fs::create_dir_all(legacy.root().join("runtime")).expect("runtime");
    let java = legacy.root().join("runtime").join("java.exe");
    fs::write(&java, b"java").expect("java");
    let profile = legacy
        .instance_dir("Survival")
        .expect("instance")
        .join("instance.json");
    fs::create_dir_all(profile.parent().expect("profile parent")).expect("instance directory");
    let original =
        serde_json::to_vec(&serde_json::json!({"javaPath": java})).expect("profile JSON");
    fs::write(&profile, &original).expect("profile");
    crate::settings::testing::save_settings_at(
        &legacy,
        &crate::settings::load_settings_at(&legacy)
            .await
            .expect("defaults"),
    )
    .await
    .expect("settings");
    // A settings file left by an earlier attempt, before any database was published.
    fs::create_dir_all(home.root()).expect("home");
    fs::write(
        home.root().join("settings.json"),
        r#"{"schema":1,"username":"Different"}"#,
    )
    .expect("conflicting settings");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert!(!db::db_path(&home).exists());
    assert_eq!(fs::read(&profile).expect("original profile"), original);
    fs::remove_file(home.root().join("settings.json")).expect("resolve conflict");
    initialize_at(&legacy, &home)
        .await
        .expect("retry migration");
    assert!(!profile.exists(), "the verified legacy profile is removed");
    assert!(db::db_path(&home).is_file());
}

#[tokio::test]
async fn migration_retries_after_its_settings_file_was_published_without_the_database() {
    let (legacy, home) = fixture("published-settings");
    let mut settings = crate::settings::load_settings_at(&legacy)
        .await
        .expect("defaults");
    settings.username = "Alex".to_string();
    settings.download_concurrency = 3;
    let legacy_file = serde_json::to_value(&settings).expect("settings JSON");
    fs::create_dir_all(legacy.root()).expect("legacy");
    fs::write(
        legacy.root().join("settings.json"),
        serde_json::to_vec(&legacy_file).expect("legacy JSON"),
    )
    .expect("legacy settings");
    // An earlier attempt stopped after publishing settings.json and before licht.db.
    let published = legacy_file
        .as_object()
        .expect("object")
        .iter()
        .filter(|(key, _)| {
            [
                "schema",
                "theme",
                "language",
                "hardwareAcceleration",
                "minimizeOnGameStart",
                "restoreOnGameExit",
                "automaticUpdates",
            ]
            .contains(&key.as_str())
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    fs::create_dir_all(home.root()).expect("home");
    fs::write(
        home.root().join("settings.json"),
        serde_json::to_vec(&published).expect("published JSON"),
    )
    .expect("published settings");
    initialize_at(&legacy, &home)
        .await
        .expect("retry migration");
    let migrated = crate::settings::load_settings_at(&home)
        .await
        .expect("migrated settings");
    assert_eq!(migrated.username, "Alex");
    assert_eq!(migrated.download_concurrency, 3);
}

#[tokio::test]
async fn concurrent_initializations_share_one_complete_migration() {
    let (legacy, home) = fixture("concurrent");
    fs::create_dir_all(legacy.root().join("versions")).expect("versions");
    fs::write(
        legacy.root().join("versions").join("fixture.json"),
        b"fixture",
    )
    .expect("fixture");
    let (first, second) =
        tokio::join!(initialize_at(&legacy, &home), initialize_at(&legacy, &home));
    first.expect("first initialization");
    second.expect("second initialization");
    assert_eq!(
        fs::read(
            home.root()
                .join("data")
                .join("versions")
                .join("fixture.json")
        )
        .expect("migrated fixture"),
        b"fixture"
    );
    assert!(db::db_path(&home).is_file());
}

#[tokio::test]
async fn migration_does_not_duplicate_instance_payloads_in_temporary_cache() {
    let (legacy, home) = fixture("no-staged-world");
    crate::settings::testing::save_settings_at(
        &legacy,
        &crate::settings::load_settings_at(&legacy)
            .await
            .expect("settings"),
    )
    .await
    .expect("legacy settings");
    let source = legacy
        .instance_dir("Survival")
        .expect("source")
        .join("saves")
        .join("level.dat");
    fs::create_dir_all(source.parent().expect("parent")).expect("save directory");
    fs::write(&source, b"original world").expect("world");
    let conflict = home
        .root()
        .join("instances")
        .join("Survival")
        .join("saves")
        .join("level.dat");
    fs::create_dir_all(conflict.parent().expect("parent")).expect("destination");
    fs::write(&conflict, b"different world").expect("conflict");
    assert!(initialize_at(&legacy, &home).await.is_err());
    fn contains_world(path: &Path) -> bool {
        fs::read_dir(path).expect("cache directory").any(|item| {
            let item = item.expect("cache entry");
            item.file_name() == "level.dat"
                || (item.file_type().expect("file type").is_dir() && contains_world(&item.path()))
        })
    }
    assert!(
        !contains_world(&home.root().join("cache")),
        "migration must not stage entire instance payloads"
    );
    assert_eq!(
        fs::read(source).expect("source preserved"),
        b"original world"
    );
    assert_eq!(
        fs::read(conflict).expect("destination preserved"),
        b"different world"
    );
}

#[test]
fn migration_copies_preserve_modification_time_and_executable_permissions() {
    let (legacy, home) = fixture("file-metadata");
    fs::create_dir_all(legacy.root()).expect("legacy directory");
    let source = legacy.root().join("java");
    let destination = home.root().join("data").join("java");
    fs::write(&source, b"executable fixture").expect("source");
    let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    fs::File::options()
        .write(true)
        .open(&source)
        .expect("source handle")
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("source time");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o755))
            .expect("executable permissions");
    }
    files::copy_file(&source, &destination).expect("copy");
    assert_eq!(
        fs::metadata(&destination)
            .expect("metadata")
            .modified()
            .expect("time"),
        modified
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&destination)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    files::copy_file(&source, &destination).expect("repeat verified copy");
    assert_eq!(
        fs::read(&destination).expect("destination"),
        b"executable fixture"
    );
}

#[tokio::test]
async fn startup_progress_can_stop_migration_before_publishing_and_retry() {
    let (legacy, home) = fixture("progress-cancellation");
    fs::create_dir_all(legacy.root().join("versions")).expect("versions");
    fs::write(
        legacy.root().join("versions").join("fixture.json"),
        b"fixture",
    )
    .expect("fixture");
    let stopped = StartupReporter::new(|progress| {
        if progress.phase == StartupPhase::MigratingInstances {
            return Err(std::io::Error::other("stopped").into());
        }
        Ok(())
    });
    assert!(
        super::initialize_at_with_progress(Some(&legacy), &home, &stopped)
            .await
            .is_err()
    );
    assert!(!db::db_path(&home).exists());
    assert_eq!(
        fs::read(legacy.root().join("versions").join("fixture.json")).expect("original"),
        b"fixture"
    );
    initialize_at(&legacy, &home).await.expect("retry");
    assert!(db::db_path(&home).exists());
}

/// Writes the settings file a 0.2.3 profile kept every setting in.
async fn write_legacy_settings(legacy: &SharedCache) {
    let settings = crate::settings::load_settings_at(legacy)
        .await
        .expect("defaults");
    fs::create_dir_all(legacy.root()).expect("legacy directory");
    fs::write(
        legacy.root().join("settings.json"),
        serde_json::to_vec(&settings).expect("settings JSON"),
    )
    .expect("legacy settings");
}

#[tokio::test]
async fn migration_removes_the_verified_legacy_profile_and_its_empty_folders() {
    let (legacy, home) = fixture("legacy-cleanup");
    write_legacy_settings(&legacy).await;
    legacy_database(&legacy)
        .await
        .close()
        .await
        .expect("close legacy database");
    // Creating the schema with the current code leaves a lock that 0.2.3 never wrote.
    fs::remove_dir_all(legacy.root().join("cache")).expect("schema lock");
    fs::create_dir_all(legacy.root().join("runtime")).expect("runtime");
    let java = legacy.root().join("runtime").join("java.exe");
    fs::write(&java, b"java").expect("java");
    fs::create_dir_all(legacy.root().join("content-metadata")).expect("metadata");
    fs::write(legacy.root().join("content-metadata/mod.json"), b"{}").expect("metadata file");
    let instance = legacy.instance_dir("Survival").expect("instance");
    fs::create_dir_all(instance.join("saves")).expect("saves");
    fs::write(instance.join("saves").join("level.dat"), b"world").expect("world");
    fs::write(
        instance.join("instance.json"),
        serde_json::to_vec(&serde_json::json!({"javaPath": java})).expect("profile"),
    )
    .expect("profile file");

    initialize_at(&legacy, &home).await.expect("migrate");

    let game = crate::settings::testing::game_cache_at(&home)
        .await
        .expect("game");
    assert_eq!(
        fs::read(game.root().join("runtime/java.exe")).expect("java"),
        b"java"
    );
    assert_eq!(
        fs::read(home.root().join("cache/content-metadata/mod.json")).expect("metadata"),
        b"{}"
    );
    assert_eq!(
        fs::read(
            game.instance_dir("Survival")
                .expect("instance")
                .join("saves/level.dat")
        )
        .expect("world"),
        b"world"
    );
    let licht = legacy
        .root()
        .parent()
        .and_then(Path::parent)
        .expect("legacy Licht folder");
    assert!(!licht.exists(), "the emptied legacy folders are removed");
    assert!(
        licht.parent().expect("fixture root").is_dir(),
        "cleanup stops at the legacy Licht folder"
    );
}

#[tokio::test]
async fn startup_cleans_an_already_migrated_profile_and_keeps_what_differs() {
    let (legacy, home) = fixture("legacy-cleanup-later");
    initialize_at(&legacy, &home).await.expect("new profile");
    // A profile migrated before cleanup existed still holds its legacy copy.
    write_legacy_settings(&legacy).await;
    for (cache, world) in [(&legacy, b"old world"), (&home, b"new world")] {
        let game = if cache == &home {
            SharedCache::at(home.root().join("data"))
        } else {
            cache.clone()
        };
        fs::create_dir_all(game.root().join("versions")).expect("versions");
        fs::write(game.root().join("versions/same.json"), b"same").expect("version");
        let saves = game
            .instance_dir("Survival")
            .expect("instance")
            .join("saves");
        fs::create_dir_all(&saves).expect("saves");
        fs::write(saves.join("level.dat"), world).expect("world");
    }
    fs::create_dir_all(legacy.root().join("logs")).expect("logs");
    fs::write(legacy.root().join("logs/latest.log"), b"log").expect("log");

    initialize_at(&legacy, &home).await.expect("startup");

    assert!(!legacy.root().join("versions").exists());
    assert!(!legacy.root().join("settings.json").exists());
    assert_eq!(
        fs::read(
            legacy
                .instance_dir("Survival")
                .expect("instance")
                .join("saves/level.dat")
        )
        .expect("changed world kept"),
        b"old world"
    );
    assert_eq!(
        fs::read(legacy.root().join("logs/latest.log")).expect("unmigrated file kept"),
        b"log"
    );
    assert_eq!(
        fs::read(home.root().join("data/versions/same.json")).expect("new copy"),
        b"same"
    );
}

#[tokio::test]
async fn a_stopped_legacy_cleanup_keeps_the_profile_and_retries_on_the_next_startup() {
    let (legacy, home) = fixture("legacy-cleanup-retry");
    write_legacy_settings(&legacy).await;
    fs::create_dir_all(legacy.root().join("versions")).expect("versions");
    fs::write(legacy.root().join("versions/fixture.json"), b"fixture").expect("fixture");
    let stopped = std::sync::atomic::AtomicBool::new(false);
    let reporter = StartupReporter::new(move |progress| {
        if progress.phase == StartupPhase::CleaningLegacyData
            && !stopped.swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(std::io::Error::other("stopped").into());
        }
        Ok(())
    });

    initialize_at_with_progress(Some(&legacy), &home, &reporter)
        .await
        .expect("startup continues")
        .close()
        .await;

    assert!(db::db_path(&home).is_file());
    assert!(legacy.root().join("settings.json").is_file());
    assert!(legacy.root().join("versions/fixture.json").is_file());
    initialize_at(&legacy, &home).await.expect("retry");
    assert!(!legacy.root().exists());
}

/// A profile as 0.2.3 wrote it beside each instance.
fn legacy_profile(name: &str, java: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "name": name,
        "versionId": "1.20.1",
        "minMemoryMb": 512,
        "maxMemoryMb": 2048,
        "jvmArguments": ["-Dlegacy=true"],
        "javaPath": java,
    })
}

async fn listed_instances(home: &SharedCache) -> Vec<crate::InstanceEntry> {
    let service =
        crate::DatabaseService::start(crate::Database::open(home).await.expect("database"))
            .await
            .expect("service");
    let game = crate::settings::testing::game_cache_at(home)
        .await
        .expect("game");
    let listed = crate::list_instances(&service.client(), &game.instances_dir().expect("dir"))
        .await
        .expect("list");
    service.shutdown().await.expect("close service");
    listed
}

#[tokio::test]
async fn migration_registers_legacy_instance_profiles_and_their_icons() {
    let (legacy, home) = fixture("legacy-profiles");
    write_legacy_settings(&legacy).await;
    fs::create_dir_all(legacy.root().join("runtime")).expect("runtime");
    let java = legacy.root().join("runtime").join("java.exe");
    fs::write(&java, b"java").expect("java");
    let instance = legacy.instance_dir("Survival").expect("instance");
    fs::create_dir_all(instance.join("saves")).expect("saves");
    fs::write(
        instance.join("instance.json"),
        serde_json::to_vec(&legacy_profile("Survival World", Some(&java))).expect("profile"),
    )
    .expect("profile file");
    fs::write(instance.join(crate::ICON_FILE), b"icon").expect("icon");
    // 0.2.3 wrote a profile for every folder it listed, so one without it is not an instance.
    fs::create_dir_all(legacy.instance_dir("Notes").expect("notes")).expect("notes folder");

    initialize_at(&legacy, &home).await.expect("migrate");

    let listed = listed_instances(&home).await;
    assert_eq!(listed.len(), 1, "only the profiled folder is an instance");
    let entry = &listed[0];
    assert_eq!(entry.folder, "Survival");
    assert_eq!(entry.name, "Survival World");
    assert_eq!(entry.version_id.as_deref(), Some("1.20.1"));
    assert_eq!(entry.jvm_arguments, ["-Dlegacy=true"]);
    let game = SharedCache::at(home.root().join("data"));
    assert_eq!(
        entry.java_path.as_deref().map(PathBuf::from),
        Some(game.root().join("runtime").join("java.exe"))
    );
    let migrated = game.instance_dir("Survival").expect("migrated instance");
    assert!(!migrated.join("instance.json").exists());
    assert!(!migrated.join(crate::ICON_FILE).exists());
    assert_eq!(
        fs::read(entry.icon_path.as_deref().expect("cached icon")).expect("icon"),
        b"icon"
    );
}

#[tokio::test]
async fn startup_registers_profiles_left_by_an_earlier_migration_without_overwriting() {
    let (legacy, home) = fixture("legacy-profiles-later");
    // A 0.3.0 profile: the database exists and migrated folders kept their profile files.
    crate::settings::testing::save_settings_at(
        &home,
        &crate::settings::load_settings_at(&home)
            .await
            .expect("defaults"),
    )
    .await
    .expect("profile");
    let instances = home.root().join("instances");
    for (folder, body) in [
        (
            "Survival",
            serde_json::to_vec(&legacy_profile("Survival World", None)).expect("profile"),
        ),
        ("Broken", b"{ not json".to_vec()),
        (
            "Kept",
            serde_json::to_vec(&legacy_profile("Stale", None)).expect("profile"),
        ),
    ] {
        fs::create_dir_all(instances.join(folder)).expect("instance");
        fs::write(instances.join(folder).join("instance.json"), body).expect("profile file");
    }
    let database = Database::open(&home).await.expect("database");
    let registered = crate::parse_instance(
        &serde_json::to_string(&legacy_profile("Registered", None)).expect("profile"),
    )
    .expect("registered profile");
    crate::store::repositories::instances::InstanceRepository::write(
        &mut database.acquire().await.expect("connection"),
        instances.to_str().expect("path"),
        "Kept",
        &registered,
    )
    .await
    .expect("register");
    database.close().await;

    initialize_at(&legacy, &home).await.expect("startup");

    let listed = listed_instances(&home).await;
    let names = listed
        .iter()
        .map(|entry| (entry.folder.as_str(), entry.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [("Kept", "Registered"), ("Survival", "Survival World")]
    );
    assert!(!instances.join("Survival/instance.json").exists());
    assert!(
        instances.join("Broken/instance.json").is_file(),
        "an unreadable profile stays for the user"
    );
    assert!(instances.join("Kept/instance.json").is_file());
}

#[tokio::test]
async fn startup_migration_previews_use_temporary_fixtures_for_success_and_conflict() {
    let snapshots = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output = std::sync::Arc::clone(&snapshots);
    let reporter = StartupReporter::new(move |snapshot| {
        output.lock().expect("snapshots").push(snapshot);
        Ok(())
    });
    crate::run_startup_migration_preview(false, reporter)
        .await
        .expect("preview migration");
    assert!(
        snapshots
            .lock()
            .expect("snapshots")
            .iter()
            .any(|snapshot| snapshot.phase == StartupPhase::MigratingInstances)
    );
    assert!(
        snapshots
            .lock()
            .expect("snapshots")
            .iter()
            .any(|snapshot| snapshot.phase == StartupPhase::CleaningLegacyData)
    );
    assert!(
        crate::run_startup_migration_preview(true, StartupReporter::default())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn startup_moves_settings_from_the_file_into_the_database() {
    let (legacy, home) = fixture("settings-to-database");
    fs::create_dir_all(home.root()).expect("home");
    fs::write(
        home.root().join("settings.json"),
        r#"{"schema":1,"username":"Alex","downloadConcurrency":4,"theme":"light"}"#,
    )
    .expect("settings with database keys");
    initialize_at(&legacy, &home).await.expect("initialize");
    let file: serde_json::Value =
        serde_json::from_slice(&fs::read(home.root().join("settings.json")).expect("file"))
            .expect("JSON");
    assert!(file.get("username").is_none());
    assert_eq!(file["theme"], "light");
    let settings = crate::settings::load_settings_at(&home)
        .await
        .expect("settings");
    assert_eq!(settings.username, "Alex");
    assert_eq!(settings.download_concurrency, 4);
    let mut connection = db::open_migrated(&home).await.expect("database");
    let stored: String = sqlx::query_scalar("SELECT username FROM settings")
        .fetch_one(&mut connection)
        .await
        .expect("stored row");
    connection.close().await.expect("close");
    assert_eq!(stored, "Alex");
}

#[tokio::test]
async fn startup_reads_update_preferences_before_migration_and_prefers_the_current_settings() {
    let (legacy, home) = fixture("startup-preferences");
    let mut old = crate::settings::load_settings_at(&legacy)
        .await
        .expect("defaults");
    old.automatic_updates = false;
    crate::settings::testing::save_settings_at(&legacy, &old)
        .await
        .expect("legacy preferences");
    let loaded = super::startup_preferences_at(&home, &legacy).expect("legacy fallback");
    assert!(!loaded.automatic_updates);
    assert!(
        !home.root().exists(),
        "reading preferences must not start migration"
    );
    let current = crate::settings::load_settings_at(&home)
        .await
        .expect("current defaults");
    crate::settings::testing::save_settings_at(&home, &current)
        .await
        .expect("current preferences");
    let loaded = super::startup_preferences_at(&home, &legacy).expect("current settings");
    assert!(loaded.automatic_updates);
}

/// A connection that has used the WAL keeps a shared lock on the database file, so an
/// exclusive-locking connection can read it only when no such connection is open.
async fn held_open(path: &std::path::Path) -> bool {
    let probe = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .locking_mode(sqlx::sqlite::SqliteLockingMode::Exclusive)
            .busy_timeout(std::time::Duration::ZERO),
    )
    .await;
    let Ok(mut probe) = probe else {
        return true;
    };
    let read = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sqlite_master")
        .fetch_one(&mut probe)
        .await;
    probe.close().await.expect("close probe");
    read.is_err()
}

#[tokio::test]
async fn an_open_pool_holds_the_database_until_it_closes() {
    let (_, home) = fixture("pool-close");
    let database = Database::open(&home).await.expect("database");
    let mut connection = database.acquire().await.expect("connection");
    sqlx::query("INSERT INTO meta VALUES ('pool', 'open')")
        .execute(&mut *connection)
        .await
        .expect("write");
    drop(connection);
    assert!(held_open(&db::db_path(&home)).await);
    database.close().await;
    assert!(!held_open(&db::db_path(&home)).await);
}

#[tokio::test]
async fn the_legacy_migration_publishes_a_database_nothing_holds_open() {
    let (legacy, home) = fixture("migration-closed");
    let settings = crate::settings::load_settings_at(&legacy)
        .await
        .expect("defaults");
    fs::create_dir_all(legacy.root()).expect("legacy directory");
    fs::write(
        legacy.root().join("settings.json"),
        serde_json::to_vec(&settings).expect("settings JSON"),
    )
    .expect("legacy settings");

    migration::migrate(&legacy, &home, &StartupReporter::default())
        .await
        .expect("migrate");

    assert!(db::db_path(&home).is_file());
    assert!(!home.root().join("cache/layout-migration").exists());
    assert!(!held_open(&db::db_path(&home)).await);
}

#[tokio::test]
async fn startup_returns_the_open_profile_pool() {
    let (legacy, home) = fixture("startup-pool");
    let database = initialize_at_with_progress(Some(&legacy), &home, &StartupReporter::default())
        .await
        .expect("initialize");
    assert_eq!(database.cache(), &home);
    assert!(held_open(&db::db_path(&home)).await);
    database.close().await;
    assert!(!held_open(&db::db_path(&home)).await);
}
