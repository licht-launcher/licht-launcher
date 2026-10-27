use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;

use super::{archive, artifacts, processors, receipt};
use crate::{CoreError, OsName, SharedCache};

#[test]
fn modpack_build_without_the_game_prefix_is_accepted() {
    let request = super::ForgeRequest {
        family: super::ForgeFamily::Forge,
        build: "47.3.0".into(),
        game_version: "1.20.1".into(),
    };
    assert!(request.validate().is_ok());
    let url = request
        .family
        .installer_url(&request.build, &request.game_version);
    assert_eq!(
        url,
        super::ForgeFamily::Forge.installer_url("1.20.1-47.3.0", "1.20.1")
    );
    assert!(url.ends_with("/forge-1.20.1-47.3.0-installer.jar"));
}

fn directory() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "licht loader tests {}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("mkdir");
    root
}

fn jar(path: &Path, entries: &[(&str, &[u8])]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    let mut writer = zip::ZipWriter::new(fs::File::create(path).expect("create jar"));
    for (name, bytes) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .expect("entry");
        writer.write_all(bytes).expect("bytes");
    }
    writer.finish().expect("zip");
}

fn profile_installer(path: &Path, resolved_hash: bool) -> archive::Installer {
    let mut library = serde_json::json!({
        "name": "fixture:library:1",
        "url": "https://libraries.example.invalid/"
    });
    if resolved_hash {
        library["sha1"] = serde_json::json!(crate::hashes::sha1_hex(b"library"));
    }
    archive::Installer {
        profile: serde_json::from_str(r#"{"minecraft":"1.20.1"}"#).expect("profile"),
        version: serde_json::json!({"mainClass":"fixture.Main", "libraries":[library]}),
        archive: zip::ZipArchive::new(fs::File::open(path).expect("installer")).expect("zip"),
    }
}

fn offline_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:1").expect("local proxy"))
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .expect("offline client")
}

#[tokio::test]
async fn prepared_forge_profiles_are_reused_without_resolving_checksums_again() {
    for family in [super::ForgeFamily::Forge, super::ForgeFamily::NeoForge] {
        let root = directory();
        let cache = SharedCache::at(&root);
        let path = root.join("installer.jar");
        jar(&path, &[("unused", b"fixture")]);
        let build = match family {
            super::ForgeFamily::Forge => "47.3.0",
            super::ForgeFamily::NeoForge => "20.1.0",
        };
        let request = super::ForgeRequest {
            family,
            build: build.into(),
            game_version: "1.20.1".into(),
        };
        request.validate().expect("valid loader request");
        let mut initial = profile_installer(&path, true);
        let client = offline_client();
        let expected =
            super::write_profile(&client, &cache, "forge-fixture", &request, &mut initial)
                .await
                .expect("initial normalization");
        assert!(!super::installation_ready(&cache, "forge-fixture"));
        let profile_path = cache.version_json("forge-fixture").expect("profile path");
        let body = fs::read(&profile_path).expect("prepared profile");
        let modified = fs::metadata(&profile_path)
            .expect("profile metadata")
            .modified()
            .expect("mtime");
        let mut reopened = profile_installer(&path, false);
        let reused =
            super::write_profile(&client, &cache, "forge-fixture", &request, &mut reopened)
                .await
                .expect("reuse the resolved profile without another checksum request");
        assert_eq!(reused.len(), expected.len());
        assert_eq!(reused[0].sha1, expected[0].sha1);
        assert_eq!(reused[0].url, expected[0].url);
        assert!(!super::installation_ready(&cache, "forge-fixture"));
        assert_eq!(fs::read(&profile_path).expect("profile"), body);
        assert_eq!(
            fs::metadata(&profile_path)
                .expect("metadata")
                .modified()
                .expect("mtime"),
            modified
        );
        drop(initial);
        drop(reopened);
        fs::remove_dir_all(root).expect("cleanup");
    }
}

#[tokio::test]
async fn tampered_prepared_profile_is_rebuilt_from_the_installer() {
    let root = directory();
    let cache = SharedCache::at(&root);
    let path = root.join("installer.jar");
    jar(&path, &[("unused", b"fixture")]);
    let request = super::ForgeRequest {
        family: super::ForgeFamily::Forge,
        build: "47.3.0".into(),
        game_version: "1.20.1".into(),
    };
    let client = offline_client();
    let mut initial = profile_installer(&path, true);
    super::write_profile(&client, &cache, "forge-fixture", &request, &mut initial)
        .await
        .expect("initial profile");
    let profile_path = cache.version_json("forge-fixture").expect("path");
    let original = fs::read(&profile_path).expect("original profile");
    fs::write(&profile_path, b"corrupt profile").expect("tampered cache");
    let mut reopened = profile_installer(&path, true);
    super::write_profile(&client, &cache, "forge-fixture", &request, &mut reopened)
        .await
        .expect("rebuild");
    assert_eq!(fs::read(&profile_path).expect("rebuilt profile"), original);
    let mut reopened_without_hash = profile_installer(&path, false);
    super::write_profile(
        &client,
        &cache,
        "forge-fixture",
        &request,
        &mut reopened_without_hash,
    )
    .await
    .expect("reuse rebuilt cache");
    drop(initial);
    drop(reopened);
    drop(reopened_without_hash);
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn prepared_profile_cache_requires_a_matching_request_and_valid_checksum() {
    let root = directory();
    let cache = SharedCache::at(&root);
    let path = root.join("installer.jar");
    jar(&path, &[("unused", b"fixture")]);
    let mut request = super::ForgeRequest {
        family: super::ForgeFamily::Forge,
        build: "47.3.0".into(),
        game_version: "1.20.1".into(),
    };
    let mut initial = profile_installer(&path, true);
    super::write_profile(
        &offline_client(),
        &cache,
        "forge-fixture",
        &request,
        &mut initial,
    )
    .await
    .expect("profile");
    assert!(
        receipt::prepared_profile(&cache, "forge-fixture", &request)
            .expect("cache")
            .is_some()
    );
    request.build = "47.3.1".into();
    assert!(
        receipt::prepared_profile(&cache, "forge-fixture", &request)
            .expect("different build")
            .is_none()
    );
    request.build = "47.3.0".into();
    let checksum = cache
        .version_json("forge-fixture")
        .expect("path")
        .with_extension("sha1");
    fs::write(&checksum, "a".repeat(40)).expect("wrong hash");
    assert!(
        receipt::prepared_profile(&cache, "forge-fixture", &request)
            .expect("invalid checksum")
            .is_none()
    );
    fs::remove_file(checksum).expect("remove checksum");
    assert!(
        receipt::prepared_profile(&cache, "forge-fixture", &request)
            .expect("unverified profile")
            .is_none()
    );
    drop(initial);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn receipt_detects_tampered_libraries_client_and_profiles() {
    let root = directory();
    let cache = SharedCache::at(&root);
    let profile = cache.version_json("forge-fixture").expect("profile");
    fs::create_dir_all(profile.parent().expect("parent")).expect("mkdir");
    fs::write(&profile, b"{}").expect("metadata");
    let client = cache.version_jar("forge-fixture").expect("client");
    fs::write(&client, b"client").expect("client");
    let library = cache
        .library("demo/library/1/library-1.jar")
        .expect("library");
    fs::create_dir_all(library.parent().expect("parent")).expect("mkdir");
    fs::write(&library, b"verified library").expect("library");
    assert!(!receipt::ready(&cache, "forge-fixture"));
    receipt::record(&cache, "forge-fixture", std::slice::from_ref(&library)).expect("receipt");
    assert!(receipt::ready(&cache, "forge-fixture"));
    fs::write(&library, b"corrupt library").expect("corrupt");
    assert!(!receipt::ready(&cache, "forge-fixture"));
    fs::write(&library, b"verified library").expect("restore");
    fs::write(&client, b"corrupt client").expect("corrupt");
    assert!(!receipt::ready(&cache, "forge-fixture"));
    fs::write(&client, b"client").expect("restore");
    fs::write(&profile, b"{\"modified\":true}").expect("change profile");
    assert!(!receipt::ready(&cache, "forge-fixture"));
    fs::remove_dir_all(root).expect("cleanup");
}

fn launch_fixture() -> (SharedCache, crate::Version) {
    let cache = SharedCache::at(directory());
    let mut profile: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/version-1.20.6.json"))
            .expect("Mojang fixture");
    profile["id"] = serde_json::json!("forge-fixture");
    profile["mainClass"] = serde_json::json!("fixture.Main");
    profile["lichtLoader"] =
        serde_json::json!({"family":"forge","build":"50.0.1","gameVersion":"1.20.6"});
    profile["libraries"] = serde_json::json!([{"name":"demo:library:1","downloads":{"artifact":{
        "path":"demo/library/1/library-1.jar","url":"https://example.invalid/library.jar","sha1":crate::hashes::sha1_hex(b"library"),"size":7
    }}}]);
    let path = cache.version_json("forge-fixture").expect("profile path");
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(&path, serde_json::to_vec(&profile).expect("profile JSON")).expect("profile");
    fs::write(
        cache.version_jar("forge-fixture").expect("client"),
        b"client",
    )
    .expect("client");
    let library = cache
        .library("demo/library/1/library-1.jar")
        .expect("library");
    fs::create_dir_all(library.parent().expect("parent")).expect("directory");
    fs::write(&library, b"library").expect("library");
    receipt::record(&cache, "forge-fixture", &[library]).expect("receipt");
    let version = crate::parse_version(&profile.to_string()).expect("version");
    (cache, version)
}

#[test]
fn direct_forge_launch_does_not_rehash_installed_binaries() {
    let (cache, version) = launch_fixture();
    fs::write(
        cache.version_jar("forge-fixture").expect("client"),
        b"changed client",
    )
    .expect("changed client");
    fs::write(
        cache
            .library("demo/library/1/library-1.jar")
            .expect("library"),
        b"changed library",
    )
    .expect("changed library");
    assert!(
        !receipt::ready(&cache, "forge-fixture"),
        "full installation/Repair validation must still detect tampering"
    );
    for os in [OsName::Windows, OsName::Linux] {
        let environment = crate::LaunchEnvironment {
            os,
            arch: crate::Arch::X86_64,
            os_version: String::new(),
            features: BTreeMap::new(),
        };
        assert!(crate::is_version_ready(
            &cache,
            "forge-fixture",
            &environment
        ));
        let command = crate::launch_command(
            Path::new("java"),
            &cache,
            "forge-fixture",
            &version,
            &environment,
            &BTreeMap::new(),
        )
        .expect("lightweight direct launch check");
        assert!(command.iter().any(|argument| argument == "fixture.Main"));
    }
    fs::remove_dir_all(cache.root()).expect("cleanup");
}

#[test]
fn direct_forge_launch_rejects_missing_outputs_or_changed_profile_metadata() {
    let (cache, version) = launch_fixture();
    let environment = crate::LaunchEnvironment {
        os: OsName::Windows,
        arch: crate::Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    let library = cache
        .library("demo/library/1/library-1.jar")
        .expect("library");
    fs::remove_file(&library).expect("missing library");
    assert!(!crate::is_version_ready(
        &cache,
        "forge-fixture",
        &environment
    ));
    assert!(matches!(
        crate::launch_command(
            Path::new("java"),
            &cache,
            "forge-fixture",
            &version,
            &environment,
            &BTreeMap::new()
        ),
        Err(CoreError::LoaderInstall { .. })
    ));
    fs::write(&library, b"library").expect("restore library");
    let client = cache.version_jar("forge-fixture").expect("client");
    fs::remove_file(&client).expect("missing client");
    assert!(!crate::is_version_ready(
        &cache,
        "forge-fixture",
        &environment
    ));
    fs::write(client, b"client").expect("restore client");
    let path = cache.version_json("forge-fixture").expect("profile");
    let mut profile: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("profile")).expect("JSON");
    profile["mainClass"] = serde_json::json!("changed.Main");
    fs::write(&path, serde_json::to_vec(&profile).expect("JSON")).expect("changed profile");
    assert!(!crate::is_version_ready(
        &cache,
        "forge-fixture",
        &environment
    ));
    assert!(matches!(
        crate::launch_command(
            Path::new("java"),
            &cache,
            "forge-fixture",
            &version,
            &environment,
            &BTreeMap::new()
        ),
        Err(CoreError::LoaderInstall { .. })
    ));
    fs::remove_dir_all(cache.root()).expect("cleanup");
}

#[test]
fn direct_forge_launch_requires_a_completed_installation_receipt() {
    let (cache, version) = launch_fixture();
    let environment = crate::LaunchEnvironment {
        os: OsName::Windows,
        arch: crate::Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    let receipt = cache
        .version_json("forge-fixture")
        .expect("profile")
        .parent()
        .expect("parent")
        .join("licht-loader-install.json");
    let mut unsupported: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt).expect("receipt")).expect("JSON");
    unsupported["schema"] = serde_json::json!(99);
    let unsupported = serde_json::to_vec(&unsupported).expect("unsupported receipt");
    for invalid in [None, Some(b"{}".as_slice()), Some(unsupported.as_slice())] {
        if let Some(bytes) = invalid {
            fs::write(&receipt, bytes).expect("invalid receipt");
        } else {
            fs::remove_file(&receipt).expect("missing receipt");
        }
        assert!(!crate::is_version_ready(
            &cache,
            "forge-fixture",
            &environment
        ));
        assert!(matches!(
            crate::launch_command(
                Path::new("java"),
                &cache,
                "forge-fixture",
                &version,
                &environment,
                &BTreeMap::new()
            ),
            Err(CoreError::LoaderInstall { .. })
        ));
    }
    fs::remove_dir_all(cache.root()).expect("cleanup");
}

#[test]
fn embedded_bytes_are_checked_before_publishing_and_archive_paths_are_validated() {
    let root = directory();
    let destination = root.join("library.jar");
    let expected = crate::hashes::sha1_hex(b"verified");
    assert!(artifacts::write_verified(&destination, b"wrong", &expected).is_err());
    assert!(!destination.exists());
    artifacts::write_verified(&destination, b"verified", &expected).expect("verified extraction");
    let archive_path = root.join("installer.jar");
    jar(
        &archive_path,
        &[("data/patch", b"patch"), (r"data\..\outside", b"escape")],
    );
    let mut archive =
        zip::ZipArchive::new(fs::File::open(&archive_path).expect("archive")).expect("zip");
    assert!(archive::entry(&mut archive, "../data/patch").is_err());
    assert!(matches!(
        archive::entry(&mut archive, r"data\..\outside"),
        Err(CoreError::CachePath)
    ));
    assert_eq!(
        archive::entry(&mut archive, "/data/patch").expect("resource"),
        b"patch"
    );
    drop(archive);
    fs::remove_dir_all(root).expect("cleanup");
}

fn processor() -> archive::Processor {
    archive::Processor {
        sides: vec!["client".into()],
        jar: "demo:processor:1".into(),
        classpath: vec!["demo:dependency:1@jar".into()],
        args: vec!["--input".into(), "{ROOT}/client jar".into()],
        outputs: BTreeMap::new(),
    }
}

#[test]
fn processor_command_uses_manifest_main_class_and_platform_separator() {
    let root = directory();
    let cache = SharedCache::at(&root);
    let processor_jar = cache
        .library(&artifacts::maven_path("demo:processor:1").expect("coordinate"))
        .expect("path");
    jar(
        &processor_jar,
        &[(
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\nMain-Class: demo.Processor\r\n\r\n",
        )],
    );
    let dependency = cache
        .library("demo/dependency/1/dependency-1.jar")
        .expect("dependency");
    fs::create_dir_all(dependency.parent().expect("parent")).expect("mkdir");
    fs::write(&dependency, b"dependency").expect("dependency");
    let data = BTreeMap::from([("ROOT".into(), root.display().to_string())]);
    for (os, separator) in [(OsName::Windows, ";"), (OsName::Linux, ":")] {
        let command = processors::command(Path::new("java"), &processor(), &data, &cache, os)
            .expect("command");
        assert_eq!(
            command[3],
            format!(
                "{}{separator}{}",
                processor_jar.display(),
                dependency.display()
            )
        );
        assert_eq!(command[4], "demo.Processor");
        assert_eq!(command[6], format!("{}/client jar", root.display()));
    }
    fs::remove_file(&dependency).expect("missing dependency");
    assert!(
        processors::command(
            Path::new("java"),
            &processor(),
            &data,
            &cache,
            OsName::Linux,
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn processor_failure_is_reported_and_server_processors_are_not_run() {
    let root = directory();
    let cache = SharedCache::at(&root);
    cache.create().expect("cache");
    let archive_path = root.join("installer.jar");
    jar(&archive_path, &[("unused", b"fixture")]);
    let mut profile: archive::Profile = serde_json::from_str(
        r#"{"minecraft":"1.20.1","data":{"SERVER_ONLY":{"server":"/data/server-only"}}}"#,
    )
    .expect("server-only data is valid");
    profile.processors = vec![archive::Processor {
        sides: vec!["server".into()],
        ..processor()
    }];
    let mut installer = archive::Installer {
        profile,
        version: serde_json::json!({}),
        archive: zip::ZipArchive::new(fs::File::open(&archive_path).expect("file")).expect("zip"),
    };
    let http = reqwest::Client::new();
    assert!(
        processors::run(
            &http,
            &mut installer,
            &cache,
            Path::new("missing-java"),
            &archive_path,
            OsName::Linux,
            &crate::InstallationContext::default(),
        )
        .await
        .expect("server is skipped")
        .is_empty()
    );
    let p = processor();
    jar(
        &cache
            .library(&artifacts::maven_path(&p.jar).expect("coordinate"))
            .expect("path"),
        &[("META-INF/MANIFEST.MF", b"Main-Class: demo.Processor\r\n")],
    );
    let dependency = cache
        .library("demo/dependency/1/dependency-1.jar")
        .expect("path");
    fs::create_dir_all(dependency.parent().expect("parent")).expect("mkdir");
    fs::write(dependency, b"dependency").expect("library");
    installer.profile.processors = vec![p];
    // The Rust test executable rejects Java flags with a nonzero exit, acting as a failed JVM.
    let java = std::env::current_exe().expect("test executable");
    let error = processors::run(
        &http,
        &mut installer,
        &cache,
        &java,
        &archive_path,
        OsName::Linux,
        &crate::InstallationContext::default(),
    )
    .await
    .expect_err("failed processor");
    assert!(matches!(
        error,
        CoreError::LoaderProcessor { processor: 1, .. }
    ));
    drop(installer);
    fs::remove_dir_all(root).expect("cleanup");
}
