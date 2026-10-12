use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use licht_core::{
    Arch, Argument, Artifact, AssetIndex, CoreError, Download, GameArguments, LaunchEnvironment,
    Library, LibraryDownloads, LibraryExtract, OsName, SharedCache, Version, VersionDownloads,
    create_natives_directory, extract_natives, native_libraries, parse_version,
};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn saved_versions_pick_one_native_per_architecture() {
    let legacy = parse_version(include_str!("../fixtures/version-1.8.9.json")).expect("1.8.9");
    let modern = parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");

    let windows_64 = paths(&legacy, OsName::Windows, Arch::X86_64);
    assert!(
        windows_64
            .iter()
            .any(|path| path.ends_with("twitch-platform-6.5-natives-windows-64.jar"))
    );
    assert!(windows_64.iter().any(|path| path.ends_with(
        "lwjgl-platform-2.9.4-nightly-20150209-natives-windows.jar"
    )));
    assert!(
        windows_64
            .iter()
            .all(|path| !path.contains("natives-linux"))
    );
    assert!(
        windows_64
            .iter()
            .all(|path| !path.contains("natives-windows-32"))
    );

    let windows_32 = paths(&legacy, OsName::Windows, Arch::X86);
    assert!(
        windows_32
            .iter()
            .any(|path| path.ends_with("twitch-platform-6.5-natives-windows-32.jar"))
    );
    assert!(
        windows_32
            .iter()
            .all(|path| !path.contains("natives-windows-64"))
    );

    let modern_windows = paths(&modern, OsName::Windows, Arch::X86_64);
    assert!(
        modern_windows
            .iter()
            .any(|path| path.ends_with("jtracy-1.0.37-natives-windows.jar"))
    );
    assert!(
        modern_windows
            .iter()
            .all(|path| !path.contains("natives-windows-arm64"))
    );
    assert!(
        modern_windows
            .iter()
            .all(|path| !path.contains("natives-windows-x86"))
    );
    assert!(
        modern_windows
            .iter()
            .all(|path| !path.contains("natives-linux"))
    );

    let modern_linux = paths(&modern, OsName::Linux, Arch::X86_64);
    assert!(
        modern_linux
            .iter()
            .any(|path| path.ends_with("jtracy-1.0.37-natives-linux.jar"))
    );
    assert!(
        modern_linux
            .iter()
            .all(|path| !path.contains("natives-windows"))
    );
}

#[test]
fn extraction_keeps_the_binary_and_drops_excluded_entries() {
    let root = std::env::temp_dir().join(format!("licht-natives-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let native_path = "org/lwjgl/lwjgl/2.9.4/lwjgl-2.9.4-natives-windows.jar";
    let classpath_path = "org/lwjgl/lwjgl/2.9.4/lwjgl-2.9.4.jar";
    write_zip(
        &cache.library(native_path).expect("native path"),
        &[
            ("lwjgl.dll", b"from-the-native"),
            ("META-INF/MANIFEST.MF", b"manifest"),
        ],
    );
    write_zip(
        &cache.library(classpath_path).expect("classpath path"),
        &[("should-not-extract.txt", b"classpath")],
    );

    let version = version_with(vec![
        native_library(native_path),
        classpath_library(classpath_path),
    ]);
    let destination = root.join("run");
    extract_natives(&cache, &version, &windows(), &destination).expect("extract");

    assert_eq!(
        std::fs::read(destination.join("lwjgl.dll")).expect("dll"),
        b"from-the-native"
    );
    assert!(!destination.join("java").exists());
    assert!(!destination.join("META-INF").join("MANIFEST.MF").exists());
    assert!(!destination.join("should-not-extract.txt").exists());
}

#[test]
fn a_nested_binary_lands_in_the_version_folder() {
    let root = std::env::temp_dir().join(format!("licht-natives-nested-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let native_path = "org/lwjgl/lwjgl/3.4.3/lwjgl-3.4.3-natives-windows.jar";
    write_zip(
        &cache.library(native_path).expect("native path"),
        &[
            ("windows/x64/org/lwjgl/lwjgl.dll", b"nested-dll"),
            ("META-INF/MANIFEST.MF", b"manifest"),
        ],
    );

    let mut version = version_with(vec![modern_native_library(native_path)]);
    version.arguments = GameArguments::Modern {
        jvm: vec![Argument::Literal(
            "-Djava.library.path=${natives_directory}/java".to_string(),
        )],
        game: Vec::new(),
    };
    let destination = root.join("run");
    extract_natives(&cache, &version, &windows(), &destination).expect("extract");

    assert_eq!(
        std::fs::read(destination.join("lwjgl.dll")).expect("dll"),
        b"nested-dll"
    );
    assert!(!destination.join("java").exists());
    assert!(!destination.join("windows").exists());
}

#[test]
fn a_root_binary_stays_when_the_library_path_is_the_natives_root() {
    let root = std::env::temp_dir().join(format!("licht-natives-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let native_path = "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar";
    write_zip(
        &cache.library(native_path).expect("native path"),
        &[("lwjgl.dll", b"root-dll")],
    );
    let mut version = version_with(vec![modern_native_library(native_path)]);
    version.arguments = GameArguments::Modern {
        jvm: vec![Argument::Literal(
            "-Djava.library.path=${natives_directory}".to_string(),
        )],
        game: Vec::new(),
    };
    let destination = root.join("run");
    extract_natives(&cache, &version, &windows(), &destination).expect("extract");
    assert_eq!(
        std::fs::read(destination.join("lwjgl.dll")).expect("dll"),
        b"root-dll"
    );
    assert!(!destination.join("java").exists());
}

#[test]
fn a_path_outside_the_destination_is_rejected() {
    let root = std::env::temp_dir().join(format!("licht-natives-slip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let native_path = "org/lwjgl/lwjgl/2.9.4/lwjgl-2.9.4-natives-windows.jar";
    write_zip(
        &cache.library(native_path).expect("native path"),
        &[("../evil.dll", b"nope")],
    );
    let version = version_with(vec![native_library(native_path)]);
    let error = extract_natives(&cache, &version, &windows(), &root.join("run"))
        .expect_err("the entry leaves the destination");
    assert!(matches!(error, CoreError::NativePath));
    assert!(!root.join("evil.dll").exists());
}

#[test]
fn natives_land_under_the_version() {
    let root = std::env::temp_dir().join(format!("licht-natives-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let directory = create_natives_directory(&cache, "26.3").expect("version folder");

    assert_eq!(directory, root.join("natives").join("26.3"));
    assert!(directory.is_dir());
    assert_eq!(
        create_natives_directory(&cache, "26.3").expect("same folder"),
        directory
    );

    let error = create_natives_directory(&cache, "..").expect_err("parent id");
    assert!(matches!(error, CoreError::CachePath));
    let _ = std::fs::remove_dir_all(root);
}

fn paths(version: &Version, os: OsName, arch: Arch) -> Vec<String> {
    native_libraries(version, &environment(os, arch))
        .into_iter()
        .map(|library| library.artifact.path.clone())
        .collect()
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

fn native_library(path: &str) -> Library {
    Library {
        name: "org.lwjgl.lwjgl:lwjgl-platform:2.9.4".to_string(),
        downloads: Some(LibraryDownloads {
            artifact: None,
            classifiers: Some(BTreeMap::from([(
                "natives-windows".to_string(),
                artifact(path),
            )])),
        }),
        natives: Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows".to_string(),
        )])),
        rules: None,
        extract: Some(LibraryExtract {
            exclude: vec!["META-INF/".to_string()],
        }),
    }
}

fn modern_native_library(path: &str) -> Library {
    Library {
        name: "org.lwjgl:lwjgl:3.4.3:natives-windows".to_string(),
        downloads: Some(LibraryDownloads {
            artifact: Some(artifact(path)),
            classifiers: None,
        }),
        natives: None,
        rules: None,
        extract: Some(LibraryExtract {
            exclude: vec!["META-INF/".to_string()],
        }),
    }
}

fn classpath_library(path: &str) -> Library {
    Library {
        name: "org.lwjgl.lwjgl:lwjgl:2.9.4".to_string(),
        downloads: Some(LibraryDownloads {
            artifact: Some(artifact(path)),
            classifiers: None,
        }),
        natives: None,
        rules: None,
        extract: None,
    }
}

fn artifact(path: &str) -> Artifact {
    Artifact {
        path: path.to_string(),
        sha1: "a".repeat(40),
        size: 1,
        url: "https://example.invalid/library.jar".to_string(),
    }
}

fn version_with(libraries: Vec<Library>) -> Version {
    Version {
        arguments: GameArguments::Legacy(String::new()),
        libraries,
        asset_index: AssetIndex {
            id: "1".to_string(),
            sha1: "a".repeat(40),
            size: 0,
            total_size: 0,
            url: "https://example.invalid/index.json".to_string(),
        },
        main_class: "a.B".to_string(),
        downloads: VersionDownloads {
            client: Download {
                sha1: "b".repeat(40),
                size: 0,
                url: "https://example.invalid/client.jar".to_string(),
            },
            server: None,
        },
        java_version: None,
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("jar directory");
    }
    let file = std::fs::File::create(path).expect("jar");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        zip.start_file(*name, options).expect("entry");
        zip.write_all(bytes).expect("bytes");
    }
    zip.finish().expect("zip");
}
