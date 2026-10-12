use std::collections::BTreeMap;

use licht_core::{
    Arch, CoreError, LaunchEnvironment, OsName, SharedCache, installed_java, parse_launch_args,
    parse_version, parse_version_manifest, parse_versions_args, version_lines,
};

#[test]
fn the_saved_manifest_names_its_latest_release() {
    let manifest = parse_version_manifest(include_str!("../fixtures/version_manifest_v2.json"))
        .expect("manifest");
    let lines = version_lines(&manifest);
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("{} release", manifest.latest.release))
    );
    assert!(lines.iter().any(|line| line.ends_with(" snapshot")));
}

#[test]
fn versions_rejects_an_unknown_flag() {
    let error = parse_versions_args(&[
        "versions".to_string(),
        "--nope".to_string(),
        "x".to_string(),
    ])
    .expect_err("unknown flag");
    assert!(matches!(error, CoreError::LaunchArgs));
}

#[test]
fn launch_can_omit_java() {
    let args = parse_launch_args(&[
        "launch".to_string(),
        "--version".to_string(),
        "26.3".to_string(),
        "--username".to_string(),
        "Mateus".to_string(),
        "--game-dir".to_string(),
        "game".to_string(),
    ])
    .expect("game dir is enough");
    assert!(args.java.is_none());
}

#[test]
fn an_installed_runtime_is_used_when_launch_omits_java() {
    let root = std::env::temp_dir().join(format!("licht-runtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let version = parse_version(
        r#"{"minecraftArguments":"","libraries":[],"assetIndex":{"id":"t","sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":1,"totalSize":1,"url":"https://example.invalid/i"},"mainClass":"a.B","downloads":{"client":{"sha1":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":1,"url":"https://example.invalid/c"}},"javaVersion":{"component":"java-runtime-epsilon","majorVersion":21}}"#,
    )
    .expect("version");
    let environment = LaunchEnvironment {
        os: OsName::Windows,
        arch: Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    assert!(matches!(
        installed_java(&cache, "1.21.11", &version, &environment),
        Err(CoreError::JavaRuntimeMissing)
    ));

    let java = cache
        .root()
        .join("runtime")
        .join("temurin21-jre21.0.2-win_x64")
        .join("bin")
        .join("java.exe");
    std::fs::create_dir_all(java.parent().expect("bin")).expect("bin");
    std::fs::write(&java, b"java").expect("java");
    assert_eq!(
        installed_java(&cache, "1.21.11", &version, &environment).expect("found"),
        java
    );
    let _ = std::fs::remove_dir_all(root);
}
