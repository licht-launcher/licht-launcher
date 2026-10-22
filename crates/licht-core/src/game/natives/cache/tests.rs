use std::collections::BTreeMap;
use std::io::Write;

use super::*;
use crate::{DatabaseService, OsName};

const VERSION: &str = "native-fixture";

struct Fixture {
    root: PathBuf,
    game: SharedCache,
    home: SharedCache,
    database: Option<DatabaseService>,
}

impl Fixture {
    async fn new() -> Self {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).expect("nonce");
        let root = std::env::temp_dir().join(format!("licht-native-cache-{}", hex_encode(&nonce)));
        let game = SharedCache::at(root.join("data"));
        let home = SharedCache::at(root.join("settings"));
        let database = Some(
            DatabaseService::start(home.clone())
                .await
                .expect("database"),
        );
        Self {
            root,
            game,
            home,
            database,
        }
    }

    fn client(&self) -> DatabaseClient {
        self.database.as_ref().expect("database").client()
    }

    fn version(&self, classifier: &str, entries: &[(&str, &[u8])]) -> Version {
        let path = format!("demo/{classifier}.jar");
        let source = self.game.library(&path).expect("library path");
        fs::create_dir_all(source.parent().expect("parent")).expect("directory");
        let mut archive = zip::ZipWriter::new(File::create(&source).expect("archive"));
        for (name, bytes) in entries {
            archive
                .start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .expect("entry");
            archive.write_all(bytes).expect("body");
        }
        archive.finish().expect("zip");
        let mut version: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/version-1.20.6.json"
        )))
        .expect("Mojang fixture");
        version["libraries"] = serde_json::json!([{"name":format!("demo:native:1:{classifier}"),"downloads":{"artifact":{
            "path":path,"sha1":sha1_hex(&fs::read(&source).expect("archive bytes")),"size":fs::metadata(source).expect("archive metadata").len(),"url":"https://example.invalid/natives.jar"
        }}}]);
        crate::parse_version(&version.to_string()).expect("version")
    }

    async fn reopen(&mut self) {
        self.database
            .take()
            .expect("database")
            .shutdown()
            .await
            .expect("shutdown");
        self.database = Some(
            DatabaseService::start(self.home.clone())
                .await
                .expect("reopen"),
        );
    }

    async fn close(mut self) {
        self.database
            .take()
            .expect("database")
            .shutdown()
            .await
            .expect("shutdown");
        fs::remove_dir_all(self.root).expect("cleanup");
    }
}

fn environment(os: OsName, arch: Arch) -> LaunchEnvironment {
    LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    }
}

fn windows() -> LaunchEnvironment {
    environment(OsName::Windows, Arch::X86_64)
}

#[tokio::test]
async fn a_verified_generation_is_reused_after_reopening_without_reading_archives_again() {
    let mut fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native body")]);
    let first = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("prepare");
    let modified = fs::metadata(first.join("native.dll"))
        .expect("metadata")
        .modified()
        .expect("mtime");
    fs::remove_file(
        fixture
            .game
            .library("demo/natives-windows.jar")
            .expect("archive"),
    )
    .expect("remove archive");
    fixture.reopen().await;
    let second = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("cache hit");
    assert_eq!(second, first);
    assert_eq!(
        fs::metadata(second.join("native.dll"))
            .expect("metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    assert_eq!(
        fs::read(second.join("native.dll")).expect("native"),
        b"native body"
    );
    fixture.close().await;
}

#[tokio::test]
async fn concurrent_launches_publish_and_share_one_complete_generation() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native body")]);
    let client = fixture.client();
    let env = windows();
    let (first, second) = tokio::join!(
        prepare(&client, &fixture.game, VERSION, &version, &env),
        prepare(&client, &fixture.game, VERSION, &version, &env)
    );
    let first = first.expect("first launch");
    assert_eq!(first, second.expect("second launch"));
    assert_eq!(
        fs::read(first.join("native.dll")).expect("native"),
        b"native body"
    );
    let published = fs::read_dir(first.parent().expect("parent"))
        .expect("generations")
        .map(|entry| entry.expect("entry"))
        .filter(|entry| entry.file_type().expect("type").is_dir())
        .count();
    assert_eq!(published, 1);
    fixture.close().await;
}

#[tokio::test]
async fn jvm_work_files_do_not_trigger_reextraction_of_managed_natives() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    let first = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("first");
    // Official profiles also use natives_directory for JNA/LWJGL/Netty work files.
    fs::create_dir(first.join("jna-work")).expect("JVM work directory");
    fs::write(
        first.join("jna-work").join("runtime.dll"),
        b"runtime output",
    )
    .expect("JVM work file");
    let reused = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("reuse");
    assert_eq!(reused, first);
    assert_eq!(
        fs::read(reused.join("native.dll")).expect("managed native"),
        b"native"
    );
    assert_eq!(
        fs::read(reused.join("jna-work").join("runtime.dll")).expect("preserved work file"),
        b"runtime output"
    );
    fixture.close().await;
}

#[tokio::test]
async fn changed_archives_and_extraction_rules_create_distinct_generations() {
    let fixture = Fixture::new().await;
    let first_version = fixture.version("natives-windows", &[("native.dll", b"original")]);
    let first = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &first_version,
        &windows(),
    )
    .await
    .expect("first");
    let mut second_version = fixture.version("natives-windows", &[("native.dll", b"replacement")]);
    let second = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &second_version,
        &windows(),
    )
    .await
    .expect("changed source");
    assert_ne!(first, second);
    assert_eq!(
        fs::read(first.join("native.dll")).expect("old generation"),
        b"original"
    );
    assert_eq!(
        fs::read(second.join("native.dll")).expect("new generation"),
        b"replacement"
    );
    second_version.libraries[0].extract = Some(crate::LibraryExtract {
        exclude: vec!["native.dll".into()],
    });
    let excluded = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &second_version,
        &windows(),
    )
    .await
    .expect("changed rules");
    assert_ne!(excluded, second);
    assert!(!excluded.join("native.dll").exists());
    fixture.close().await;
}

#[tokio::test]
async fn missing_or_modified_managed_outputs_rebuild_without_overwriting_old_generations() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native body")]);
    let first = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("first");
    fs::remove_file(first.join("native.dll")).expect("remove output");
    let second = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("missing output");
    assert_ne!(first, second);
    fs::write(second.join("native.dll"), b"changed dll").expect("change same-size output");
    File::options()
        .write(true)
        .open(second.join("native.dll"))
        .expect("output")
        .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1))
        .expect("changed mtime");
    let third = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("modified output");
    assert_ne!(second, third);
    assert_eq!(
        fs::read(second.join("native.dll")).expect("old output preserved"),
        b"changed dll"
    );
    fs::write(third.join("unexpected.dll"), b"unexpected").expect("unexpected output");
    let fourth = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("unexpected output");
    assert_eq!(third, fourth);
    assert_eq!(
        fs::read(fourth.join("unexpected.dll")).expect("unmanaged output preserved"),
        b"unexpected"
    );
    fs::write(fourth.join("native.dll"), b"changed size").expect("different size");
    let fifth = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("size changed");
    assert_ne!(fourth, fifth);
    assert!(
        !fifth.join("unexpected.dll").exists(),
        "a new generation only extracts managed archive outputs"
    );
    fixture.close().await;
}

#[tokio::test]
async fn platform_architecture_and_data_directory_select_independent_caches() {
    let fixture = Fixture::new().await;
    let mut version = fixture.version("natives-windows", &[("native.dll", b"windows")]);
    let linux = fixture.version("natives-linux", &[("libnative.so", b"linux")]);
    version.libraries.extend(linux.libraries);
    let win = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("Windows");
    let linux = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &environment(OsName::Linux, Arch::X86_64),
    )
    .await
    .expect("Linux");
    let x86 = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &environment(OsName::Windows, Arch::X86),
    )
    .await
    .expect("x86");
    assert_ne!(win, linux);
    assert_ne!(win, x86);
    assert_eq!(
        fs::read(linux.join("libnative.so")).expect("Linux output"),
        b"linux"
    );
    let other = SharedCache::at(fixture.root.join("other-data"));
    for native in native_libraries(&version, &windows()) {
        let source = fixture.game.library(&native.artifact.path).expect("source");
        let destination = other.library(&native.artifact.path).expect("destination");
        fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
        fs::copy(source, destination).expect("copy fixture");
    }
    let isolated = prepare(&fixture.client(), &other, VERSION, &version, &windows())
        .await
        .expect("other directory");
    assert!(isolated.starts_with(other.root()));
    assert_ne!(isolated, win);
    fixture.close().await;
}

#[tokio::test]
async fn failed_extraction_does_not_publish_partial_data_or_a_cache_record() {
    let fixture = Fixture::new().await;
    let invalid = fixture.version(
        "natives-windows",
        &[("native.dll", b"partial"), ("../escape.dll", b"escape")],
    );
    assert!(matches!(
        prepare(
            &fixture.client(),
            &fixture.game,
            VERSION,
            &invalid,
            &windows()
        )
        .await,
        Err(CoreError::NativePath)
    ));
    let fingerprint = fingerprint(&fixture.game, &invalid, &windows()).expect("fingerprint");
    assert!(
        fixture
            .client()
            .native_generation(&fixture.game, VERSION, &fingerprint)
            .await
            .expect("cache")
            .is_none()
    );
    let parent = fixture
        .game
        .natives_dir(VERSION)
        .expect("root")
        .join(platform(&windows()));
    assert!(
        fs::read_dir(parent).expect("directory").all(|item| !item
            .expect("item")
            .file_type()
            .expect("type")
            .is_dir())
    );
    assert!(!fixture.root.join("escape.dll").exists());
    fixture.close().await;
}

#[tokio::test]
async fn a_cold_cache_rejects_archive_checksum_mismatches() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    fs::write(
        fixture
            .game
            .library("demo/natives-windows.jar")
            .expect("archive"),
        b"corrupt archive",
    )
    .expect("corrupt");
    assert!(matches!(
        prepare(
            &fixture.client(),
            &fixture.game,
            VERSION,
            &version,
            &windows()
        )
        .await,
        Err(CoreError::Sha1Mismatch { .. })
    ));
    fixture.close().await;
}

#[tokio::test]
async fn invalidation_keeps_existing_binaries_and_allows_a_fresh_generation() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    let legacy = fixture.game.natives_dir(VERSION).expect("legacy root");
    fs::create_dir_all(&legacy).expect("legacy directory");
    fs::write(legacy.join("native.dll"), b"legacy native").expect("legacy binary");
    #[cfg(windows)]
    let legacy_in_use = {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(legacy.join("native.dll"))
            .expect("simulate legacy loaded DLL")
    };
    let first = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("first");
    #[cfg(windows)]
    let in_use = {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(first.join("native.dll"))
            .expect("simulate loaded DLL")
    };
    let same = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("read-only reuse");
    assert_eq!(same, first);
    fixture
        .client()
        .invalidate_native_generations(&fixture.game, VERSION)
        .await
        .expect("invalidate");
    let second = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("new generation");
    assert_ne!(second, first);
    assert_eq!(
        fs::read(first.join("native.dll")).expect("old native"),
        b"native"
    );
    assert_eq!(
        fs::read(legacy.join("native.dll")).expect("legacy native preserved"),
        b"legacy native"
    );
    #[cfg(windows)]
    drop(in_use);
    #[cfg(windows)]
    drop(legacy_in_use);
    fixture.close().await;
}

#[tokio::test]
async fn a_database_publication_failure_leaves_no_reusable_generation() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    let mut connection = crate::open_db(&fixture.home).await.expect("connection");
    sqlx::raw_sql("CREATE TRIGGER reject_native_cache BEFORE INSERT ON native_generations BEGIN SELECT RAISE(ABORT, 'native cache fixture'); END;")
        .execute(&mut connection).await.expect("trigger");
    assert!(
        prepare(
            &fixture.client(),
            &fixture.game,
            VERSION,
            &version,
            &windows()
        )
        .await
        .is_err()
    );
    let key = fingerprint(&fixture.game, &version, &windows()).expect("key");
    assert!(
        fixture
            .client()
            .native_generation(&fixture.game, VERSION, &key)
            .await
            .expect("cache")
            .is_none()
    );
    sqlx::query("DROP TRIGGER reject_native_cache")
        .execute(&mut connection)
        .await
        .expect("allow publication");
    sqlx::Connection::close(connection).await.expect("close");
    let path = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("retry");
    assert_eq!(
        fs::read(path.join("native.dll")).expect("native"),
        b"native"
    );
    fixture.close().await;
}

#[tokio::test]
async fn offline_and_microsoft_launches_share_the_verified_generation() {
    let fixture = Fixture::new().await;
    let mut version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    version.arguments = crate::GameArguments::Legacy("--native-dir ${natives_directory}".into());
    let index = fixture
        .game
        .asset_index(&version.asset_index.id)
        .expect("asset index");
    fs::create_dir_all(index.parent().expect("parent")).expect("directory");
    fs::write(index, r#"{"objects":{}}"#).expect("index");
    let global = crate::GlobalLaunch {
        max_memory_mb: 2048,
        jvm_arguments: Vec::new(),
        fullscreen: false,
        width: None,
        height: None,
    };
    let client = fixture.client();
    let first = crate::prepare_offline_launch(
        &client,
        Path::new("java"),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
        &crate::offline_account("FixturePlayer").expect("offline account"),
        &fixture.root.join("offline"),
        &global,
    )
    .await
    .expect("offline launch");
    fs::remove_file(
        fixture
            .game
            .library("demo/natives-windows.jar")
            .expect("archive"),
    )
    .expect("cached launch must not reopen source archives");
    let session = crate::MicrosoftSession {
        profile: crate::AccountProfile {
            uuid: "00000000000000000000000000000000".into(),
            name: "FixturePlayer".into(),
        },
        msa_access_token: String::new(),
        msa_refresh_token: None,
        minecraft_token: String::new(),
        minecraft_expires_at_unix: 0,
        xuid: String::new(),
    };
    let second = crate::prepare_microsoft_launch(
        &client,
        Path::new("java"),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
        &session,
        &fixture.root.join("microsoft"),
        &global,
    )
    .await
    .expect("Microsoft launch");
    let first_arg = first
        .command
        .iter()
        .position(|argument| argument == "--native-dir")
        .expect("native argument")
        + 1;
    let second_arg = second
        .command
        .iter()
        .position(|argument| argument == "--native-dir")
        .expect("native argument")
        + 1;
    assert_eq!(first.command[first_arg], second.command[second_arg]);
    fixture.close().await;
}

#[tokio::test]
async fn invalid_database_paths_are_rebuilt_without_escaping_the_cache() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("first");
    let mut connection = crate::open_db(&fixture.home).await.expect("database");
    sqlx::query("UPDATE native_generations SET generation = '../outside'")
        .execute(&mut connection)
        .await
        .expect("invalid generation");
    let rebuilt = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("rebuild invalid reference");
    assert!(rebuilt.starts_with(fixture.game.natives_dir(VERSION).expect("root")));
    sqlx::query("UPDATE native_outputs SET name = '../outside.dll'")
        .execute(&mut connection)
        .await
        .expect("invalid output name");
    let rebuilt_again = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("rebuild invalid output metadata");
    assert_ne!(rebuilt, rebuilt_again);
    assert!(!fixture.root.join("outside.dll").exists());
    sqlx::Connection::close(connection).await.expect("close");
    fixture.close().await;
}

#[tokio::test]
async fn output_metadata_failure_rolls_back_the_previous_generation_reference() {
    let fixture = Fixture::new().await;
    let version = fixture.version("natives-windows", &[("native.dll", b"native")]);
    prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("first");
    let key = fingerprint(&fixture.game, &version, &windows()).expect("key");
    let original = fixture
        .client()
        .native_generation(&fixture.game, VERSION, &key)
        .await
        .expect("cache")
        .expect("generation");
    let parent = fixture
        .game
        .natives_dir(VERSION)
        .expect("root")
        .join(platform(&windows()));
    fs::remove_file(parent.join(&original.generation).join("native.dll")).expect("force rebuild");
    let mut connection = crate::open_db(&fixture.home).await.expect("database");
    sqlx::raw_sql("CREATE TRIGGER reject_native_output BEFORE INSERT ON native_outputs BEGIN SELECT RAISE(ABORT, 'output metadata fixture'); END;")
        .execute(&mut connection).await.expect("trigger");
    assert!(
        prepare(
            &fixture.client(),
            &fixture.game,
            VERSION,
            &version,
            &windows()
        )
        .await
        .is_err()
    );
    assert_eq!(
        fixture
            .client()
            .native_generation(&fixture.game, VERSION, &key)
            .await
            .expect("rolled back cache"),
        Some(original)
    );
    sqlx::query("DROP TRIGGER reject_native_output")
        .execute(&mut connection)
        .await
        .expect("allow publication");
    sqlx::Connection::close(connection).await.expect("close");
    let repaired = prepare(
        &fixture.client(),
        &fixture.game,
        VERSION,
        &version,
        &windows(),
    )
    .await
    .expect("retry");
    assert_eq!(
        fs::read(repaired.join("native.dll")).expect("native"),
        b"native"
    );
    fixture.close().await;
}
