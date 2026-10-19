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
