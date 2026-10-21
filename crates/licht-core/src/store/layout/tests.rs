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
    let connection = db::open_db(legacy).await.expect("create schema");
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
        crate::game_cache(&home).expect("game").root(),
        home.root().join("data")
    );
    for directory in ["cache", "data", "instances"] {
        assert!(home.root().join(directory).is_dir());
    }
    assert!(home.root().join("settings.json").is_file());
    let mut connection = db::open_db(&home).await.expect("database");
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
    let mut settings = crate::load_settings(&legacy).expect("settings");
    settings.download_concurrency = 3;
    settings.java21 = Some(artifact.to_string_lossy().into_owned());
    crate::save_settings(&legacy, &settings).expect("legacy settings");
    let instance = legacy.instance_dir("Survival").expect("instance");
    fs::create_dir_all(instance.join("saves")).expect("save directory");
    fs::write(instance.join("saves").join("level.dat"), b"world").expect("world");
    fs::write(
        instance.join("instance.json"),
        serde_json::to_vec(&serde_json::json!({"javaPath": artifact})).expect("profile"),
    )
    .expect("profile file");
    let service = crate::DatabaseService::start(legacy.clone())
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
    let new_game = crate::game_cache(&home).expect("new game");
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
    assert_eq!(fs::read(&artifact).expect("original preserved"), b"library");
    let migrated = crate::load_settings(&home).expect("migrated settings");
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
    let mut migrated = db::open_db(&home).await.expect("migrated database");
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
    let mut settings = crate::load_settings(&legacy).expect("defaults");
    settings.data_directory = Some(chosen.to_string_lossy().into_owned());
    crate::save_settings(&legacy, &settings).expect("settings");
    let old_game = SharedCache::at(chosen.join("Licht Launcher"));
    fs::create_dir_all(old_game.root().join("versions")).expect("versions");
    fs::write(old_game.root().join("versions/custom.json"), b"version").expect("version");
    let old_instance = old_game.instance_dir("Custom").expect("old instance");
    fs::create_dir_all(&old_instance).expect("instance");
    fs::write(old_instance.join("save.dat"), b"save").expect("save");
    initialize_at(&legacy, &home)
        .await
        .expect("custom migration");
    let game = crate::game_cache(&home).expect("custom game");
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
    assert!(old_instance.join("save.dat").is_file());
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
    crate::save_settings(&legacy, &crate::load_settings(&legacy).expect("defaults"))
        .expect("settings");
    let mut conflicting = crate::load_settings(&home).expect("destination defaults");
    conflicting.username = "Different".to_string();
    crate::save_settings(&home, &conflicting).expect("conflicting settings");
    assert!(initialize_at(&legacy, &home).await.is_err());
    assert!(!db::db_path(&home).exists());
    assert_eq!(fs::read(&profile).expect("original profile"), original);
    fs::remove_file(home.root().join("settings.json")).expect("resolve conflict");
    initialize_at(&legacy, &home)
        .await
        .expect("retry migration");
    assert_eq!(
        fs::read(&profile).expect("original still preserved"),
        original
    );
    assert!(db::db_path(&home).is_file());
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
    crate::save_settings(&legacy, &crate::load_settings(&legacy).expect("settings"))
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
