use std::collections::BTreeMap;
use std::path::Path;

use licht_core::{
    Arch, CoreError, LaunchEnvironment, OsName, SharedCache, classpath, launch_command,
    parse_version,
};

#[test]
fn windows_command_joins_the_classpath_with_semicolons_and_omits_native_jars() {
    let version = parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");
    let cache = SharedCache::at("game-cache");
    let environment = environment(OsName::Windows, Arch::X86_64);
    let joined = classpath(&cache, "1.21.11", &version, &environment).expect("classpath");
    let entries = joined.split(';').map(normalize).collect::<Vec<_>>();

    assert!(joined.contains(';'));
    assert!(!joined.contains(':'));
    assert!(
        entries
            .iter()
            .any(|entry| entry.ends_with("versions/1.21.11/1.21.11.jar"))
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.ends_with("lz4-java-1.8.1.jar"))
    );
    assert!(entries.iter().all(|entry| {
        !entry.ends_with("natives-windows.jar")
            && !entry.ends_with("natives-windows-x86.jar")
            && !entry.ends_with("natives-windows-arm64.jar")
    }));

    let command = launch_command(
        Path::new("runtime/java"),
        &cache,
        "1.21.11",
        &version,
        &environment,
        &values(&[("auth_player_name", "Alex")]),
    )
    .expect("command");
    let classpath_index = command.iter().position(|arg| arg == "-cp").expect("-cp");
    assert_eq!(command[classpath_index + 1], joined);
    assert_eq!(command.iter().filter(|arg| *arg == "-cp").count(), 1);
    let main_class = command
        .iter()
        .position(|arg| arg == "net.minecraft.client.main.Main")
        .expect("main class");
    assert!(main_class > classpath_index);
    let username = command
        .iter()
        .position(|arg| arg == "--username")
        .expect("username");
    assert_eq!(command[username + 1], "Alex");
    assert!(username > main_class);
    assert!(command.iter().any(|arg| arg.contains("HeapDumpPath")));
    assert!(command.iter().all(|arg| arg != "-XstartOnFirstThread"));
}

#[test]
fn linux_uses_colons_and_skips_arguments_for_other_systems() {
    let version = parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");
    let cache = SharedCache::at("game-cache");
    let linux = environment(OsName::Linux, Arch::X86_64);
    let joined = classpath(&cache, "1.21.11", &version, &linux).expect("classpath");

    assert!(joined.contains(':'));
    assert!(!joined.contains(';'));

    let osx = classpath(
        &cache,
        "1.21.11",
        &version,
        &environment(OsName::Osx, Arch::X86_64),
    )
    .expect("osx classpath");
    assert!(osx.contains(':'));
    assert!(!osx.contains(';'));

    let command = launch_command(
        Path::new("runtime/java"),
        &cache,
        "1.21.11",
        &version,
        &linux,
        &BTreeMap::new(),
    )
    .expect("command");
    assert!(command.iter().all(|arg| !arg.contains("HeapDumpPath")));
    assert!(command.iter().all(|arg| arg != "-XstartOnFirstThread"));
}

#[test]
fn legacy_command_adds_the_jvm_prefix_and_keeps_the_main_jar() {
    let version = parse_version(include_str!("../fixtures/version-1.8.9.json")).expect("1.8.9");
    let cache = SharedCache::at("game-cache");
    let environment = environment(OsName::Windows, Arch::X86_64);
    let java = Path::new("runtime/java");
    let joined = classpath(&cache, "1.8.9", &version, &environment).expect("classpath");
    let entries = joined.split(';').map(normalize).collect::<Vec<_>>();

    assert!(
        entries
            .iter()
            .any(|entry| entry.ends_with("netty-1.8.8.jar"))
    );
    assert!(
        entries
            .iter()
            .any(|entry| { entry.ends_with("lwjgl-platform-2.9.4-nightly-20150209.jar") })
    );
    assert!(entries.iter().all(|entry| {
        !entry.contains("natives-windows")
            && !entry.contains("natives-linux")
            && !entry.contains("natives-osx")
    }));

    let command = launch_command(
        java,
        &cache,
        "1.8.9",
        &version,
        &environment,
        &values(&[
            ("auth_player_name", "Steve"),
            ("natives_directory", "natives-dir"),
        ]),
    )
    .expect("command");

    assert_eq!(command[0], java.display().to_string());
    assert_eq!(command[1], "-Djava.library.path=natives-dir");
    assert_eq!(command[2], "-cp");
    assert_eq!(command[3], joined);
    assert_eq!(command[4], "net.minecraft.client.main.Main");
    let username = command
        .iter()
        .position(|arg| arg == "--username")
        .expect("username");
    assert_eq!(command[username + 1], "Steve");
}

#[test]
fn feature_rules_stay_out_until_the_feature_is_enabled() {
    let version = parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");
    let cache = SharedCache::at("game-cache");
    let mut environment = environment(OsName::Linux, Arch::X86_64);
    let values = values(&[("resolution_width", "1280"), ("resolution_height", "720")]);

    let without = launch_command(
        Path::new("runtime/java"),
        &cache,
        "1.21.11",
        &version,
        &environment,
        &values,
    )
    .expect("command");
    assert!(without.iter().all(|arg| arg != "--demo"));
    assert!(without.iter().all(|arg| arg != "--width"));
    assert!(without.iter().all(|arg| arg != "--height"));

    environment
        .features
        .insert("is_demo_user".to_string(), true);
    environment
        .features
        .insert("has_custom_resolution".to_string(), true);
    let with = launch_command(
        Path::new("runtime/java"),
        &cache,
        "1.21.11",
        &version,
        &environment,
        &values,
    )
    .expect("command");
    assert!(with.iter().any(|arg| arg == "--demo"));
    let width = with.iter().position(|arg| arg == "--width").expect("width");
    assert_eq!(with[width + 1], "1280");
    let height = with
        .iter()
        .position(|arg| arg == "--height")
        .expect("height");
    assert_eq!(with[height + 1], "720");
}

#[test]
fn an_unknown_placeholder_is_left_unchanged() {
    let version = parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");
    let command = launch_command(
        Path::new("runtime/java"),
        &SharedCache::at("game-cache"),
        "1.21.11",
        &version,
        &environment(OsName::Linux, Arch::X86_64),
        &BTreeMap::new(),
    )
    .expect("command");

    assert!(command.iter().any(|arg| arg == "${version_name}"));
    assert!(
        command
            .iter()
            .any(|arg| arg == "-Dminecraft.launcher.brand=${launcher_name}")
    );
    assert!(command.iter().all(|arg| arg != "${classpath}"));
}

#[test]
fn a_java_library_suffix_points_at_the_version_folder() {
    let version = parse_version(
        r#"{"arguments":{"jvm":["-Djava.library.path=${natives_directory}/java"],"game":[]},"libraries":[],"assetIndex":{"id":"t","sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":1,"totalSize":1,"url":"https://example.invalid/i"},"mainClass":"a.B","downloads":{"client":{"sha1":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":1,"url":"https://example.invalid/c"}}}"#,
    )
    .expect("version");
    let command = launch_command(
        Path::new("runtime/java"),
        &SharedCache::at("game-cache"),
        "26.3",
        &version,
        &environment(OsName::Windows, Arch::X86_64),
        &values(&[("natives_directory", "natives/26.3")]),
    )
    .expect("command");

    assert!(
        command
            .iter()
            .any(|arg| arg == "-Djava.library.path=natives/26.3")
    );
    assert!(
        command
            .iter()
            .all(|arg| arg != "-Djava.library.path=natives/26.3/java")
    );
}

#[test]
fn an_invalid_version_id_is_rejected() {
    let version = parse_version(include_str!("../fixtures/version-1.8.9.json")).expect("1.8.9");
    let error = classpath(
        &SharedCache::at("game-cache"),
        "..",
        &version,
        &environment(OsName::Linux, Arch::X86_64),
    )
    .expect_err("the version id must be one path component");

    assert!(matches!(error, CoreError::CachePath));
}

fn environment(os: OsName, arch: Arch) -> LaunchEnvironment {
    LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

fn normalize(entry: &str) -> String {
    entry.replace('\\', "/")
}
