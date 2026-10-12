use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use licht_core::{
    Arch, CoreError, JavaChoice, JavaPaths, JavaRuntimeEntry, LaunchEnvironment, OsName,
    SharedCache, browse_start_directory, default_java_roots, discover_javas, install_java,
    java_detect_roots, java_installation_status, matching_java, parse_java_runtime_index,
    parse_java_runtime_manifest, parse_java_version, parse_version, probe_java_major,
    required_runtime, runtime_java_paths, select_runtime, validate_java,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

#[test]
fn an_installed_runtime_java_is_offered_and_a_missing_file_is_invalid() {
    let root = std::env::temp_dir().join(format!("licht-runtime-java-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let environment = LaunchEnvironment {
        os: OsName::Windows,
        arch: Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    let executable = root
        .join("runtime")
        .join("temurin21-jre21.0.7-win_x64")
        .join("bin")
        .join("java.exe");
    std::fs::create_dir_all(executable.parent().expect("bin")).expect("runtime");
    std::fs::write(&executable, b"not a java").expect("file");

    assert!(
        java_detect_roots(&cache, OsName::Windows)
            .iter()
            .any(|path| path == &cache.root().join("runtime"))
    );
    let paths = runtime_java_paths(&cache, &environment).expect("runtime");
    assert_eq!(
        paths.java21.as_deref(),
        Some(executable.to_str().expect("utf8"))
    );
    assert!(paths.java25.is_none());
    assert!(paths.java17.is_none());
    assert!(paths.java8.is_none());

    let status = java_installation_status(&JavaPaths {
        java21: Some(root.join("missing.exe").display().to_string()),
        ..JavaPaths::default()
    });
    assert!(!status.java25);
    assert!(!status.java21);
    assert!(!status.java17);
    assert!(!status.java8);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn saved_versions_name_their_runtime() {
    for (json, component, major) in [
        (
            include_str!("../fixtures/version-1.8.9.json"),
            "jre-legacy",
            8,
        ),
        (
            include_str!("../fixtures/version-1.12.2.json"),
            "jre-legacy",
            8,
        ),
        (
            include_str!("../fixtures/version-1.20.6.json"),
            "java-runtime-delta",
            21,
        ),
        (
            include_str!("../fixtures/version-1.21.11.json"),
            "java-runtime-delta",
            21,
        ),
    ] {
        let version = parse_version(json).expect("the saved version should parse");
        let runtime = required_runtime(&version).expect("the saved version names a runtime");
        assert_eq!(runtime.component, component);
        assert_eq!(runtime.major_version, major);
    }
}

#[test]
fn a_version_without_java_version_has_no_required_runtime() {
    let version = parse_version(
        r#"{
            "minecraftArguments": "--username",
            "libraries": [],
            "assetIndex": {
                "id": "1.8",
                "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size": 1,
                "totalSize": 2,
                "url": "https://example.invalid/index.json"
            },
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "size": 3,
                    "url": "https://example.invalid/client.jar"
                }
            }
        }"#,
    )
    .expect("a legacy version without javaVersion should parse");

    assert!(required_runtime(&version).is_none());
}

#[test]
fn the_saved_runtime_index_points_at_java_21_and_java_8() {
    let index = parse_java_runtime_index(include_str!("../fixtures/java-runtime-index.json"))
        .expect("the saved runtime index should parse");

    let modern = select_runtime(&index, OsName::Windows, Arch::X86_64, "java-runtime-delta")
        .expect("Windows x64 should have Java 21");
    assert_eq!(modern.sha1, "cb4394a27089d19f65d5baa6cf0482c27c3c7865");

    let legacy = select_runtime(&index, OsName::Linux, Arch::X86, "jre-legacy")
        .expect("32-bit Linux should have the legacy runtime");
    assert_eq!(legacy.sha1.len(), 40);
    assert!(matches!(
        select_runtime(&index, OsName::Osx, Arch::X86, "jre-legacy"),
        Err(CoreError::JavaRuntimeMissing)
    ));
}

#[test]
fn the_saved_runtime_manifest_lists_java_exe() {
    let manifest = parse_java_runtime_manifest(include_str!("../fixtures/java-runtime-delta.json"))
        .expect("the saved runtime manifest should parse");
    assert!(matches!(
        manifest.files.get("bin"),
        Some(JavaRuntimeEntry::Directory)
    ));
    assert!(matches!(
        manifest.files.get("bin/java.exe"),
        Some(JavaRuntimeEntry::File {
            executable: true,
            ..
        })
    ));
}

#[tokio::test]
async fn a_custom_java_is_used_without_downloading() {
    let directory = std::env::temp_dir().join(format!("licht-java-custom-{}", std::process::id()));
    let executable = directory.join("java.exe");
    std::fs::create_dir_all(&directory).expect("temp dir");
    std::fs::write(&executable, b"custom").expect("java");
    let cache = SharedCache::at(directory.join("cache"));
    let (progress, _incoming) = mpsc::channel(1);

    let installed = install_java(
        &reqwest::Client::new(),
        &cache,
        &cache,
        JavaChoice::Custom(&executable),
        progress,
    )
    .await
    .expect("the custom java should be accepted");

    assert_eq!(installed, executable);
    let missing_path = directory.join("missing.exe");
    let missing = install_java(
        &reqwest::Client::new(),
        &cache,
        &cache,
        JavaChoice::Custom(&missing_path),
        mpsc::channel(1).0,
    )
    .await
    .expect_err("a missing java should fail");
    assert!(matches!(missing, CoreError::JavaPath));
    let _ = std::fs::remove_dir_all(directory);
}

#[tokio::test]
async fn the_mojang_runtime_is_installed_once() {
    let directory = std::env::temp_dir().join(format!("licht-java-mojang-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let manifest = parse_java_runtime_manifest(&manifest_json(&server.base))
        .expect("the synthetic manifest should parse");
    let (progress, mut incoming) = mpsc::channel(4);

    let installed = install_java(
        &reqwest::Client::new(),
        &cache,
        &cache,
        JavaChoice::Mojang {
            component: "java-runtime-delta",
            os: OsName::Windows,
            arch: Arch::X86_64,
            manifest: &manifest,
        },
        progress,
    )
    .await
    .expect("the runtime should install");

    let java = cache
        .runtime_dir("java-runtime-delta", "windows-x64")
        .expect("runtime dir")
        .join("bin")
        .join("java.exe");
    assert_eq!(installed, java);
    assert_eq!(std::fs::read(&java).expect("java.exe"), b"hello world");
    assert_eq!(
        std::fs::read(java.parent().expect("bin").join("java")).expect("link"),
        b"hello world"
    );
    while incoming.recv().await.is_some() {}
    let requests = server.requests.load(Ordering::SeqCst);
    assert!(requests >= 1);

    let (progress, _incoming) = mpsc::channel(4);
    install_java(
        &reqwest::Client::new(),
        &cache,
        &cache,
        JavaChoice::Mojang {
            component: "java-runtime-delta",
            os: OsName::Windows,
            arch: Arch::X86_64,
            manifest: &manifest,
        },
        progress,
    )
    .await
    .expect("the second install should skip the download");
    assert_eq!(server.requests.load(Ordering::SeqCst), requests);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn java_version_text_uses_the_legacy_and_modern_major() {
    assert_eq!(
        parse_java_version("java version \"1.8.0_51\"\r\n").expect("java 8"),
        8
    );
    assert_eq!(
        parse_java_version("openjdk version \"21.0.7\" 2025-04-15\n").expect("java 21"),
        21
    );
    assert_eq!(
        parse_java_version("java version \"17.0.12\" 2024-07-16\n").expect("java 17"),
        17
    );
    assert!(matches!(
        parse_java_version("not a java"),
        Err(CoreError::JavaProbe)
    ));
}

#[test]
fn search_roots_follow_the_operating_system() {
    assert!(default_java_roots(OsName::Windows).contains(&r"C:\Program Files\Java"));
    assert!(default_java_roots(OsName::Windows).contains(&r"C:\Program Files\Eclipse Adoptium"));
    assert!(default_java_roots(OsName::Linux).contains(&"/usr/lib/jvm"));
    assert!(default_java_roots(OsName::Osx).contains(&"/Library/Java/JavaVirtualMachines"));
    assert!(!default_java_roots(OsName::Windows).contains(&"/usr/lib/jvm"));
}

#[test]
fn discovery_finds_binaries_and_ignores_a_missing_root() {
    let root = std::env::temp_dir().join(format!("licht-java-discover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let windows = root.join("win").join("jdk-21").join("bin").join("java.exe");
    let linux = root
        .join("linux")
        .join("java-8-openjdk")
        .join("bin")
        .join("java");
    let macos = root
        .join("mac")
        .join("jdk-21.jdk")
        .join("Contents")
        .join("Home")
        .join("bin")
        .join("java");
    for path in [&windows, &linux, &macos] {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, b"").expect("binary");
    }
    std::fs::write(root.join("win").join("readme.txt"), b"no").expect("unrelated file");

    assert_eq!(
        discover_javas(&[root.join("win")], OsName::Windows),
        vec![windows]
    );
    assert_eq!(
        discover_javas(&[root.join("linux")], OsName::Linux),
        vec![linux]
    );
    assert_eq!(
        discover_javas(&[root.join("mac")], OsName::Osx),
        vec![macos]
    );
    assert!(discover_javas(&[root.join("missing")], OsName::Linux).is_empty());
}

#[test]
fn browse_starts_in_the_existing_folder_or_the_fallback() {
    let root = std::env::temp_dir().join(format!("licht-java-browse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let runtime = root.join("runtime");
    let bin = root.join("jdk").join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let executable = bin.join("java.exe");
    std::fs::write(&executable, b"java").expect("file");

    assert_eq!(
        browse_start_directory(executable.to_str().expect("utf8"), &runtime),
        bin
    );
    assert_eq!(
        browse_start_directory(bin.to_str().expect("utf8"), &runtime),
        bin
    );
    assert_eq!(browse_start_directory("", &runtime), runtime);
    assert_eq!(browse_start_directory("   ", &runtime), runtime);
    assert_eq!(
        browse_start_directory(
            root.join("missing")
                .join("java.exe")
                .to_str()
                .expect("utf8"),
            &runtime
        ),
        runtime
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_release_file_reports_the_major_without_running_java() {
    let root = std::env::temp_dir().join(format!("licht-java-release-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("temurin21");
    let executable = home.join("bin").join("java.exe");
    std::fs::create_dir_all(executable.parent().expect("bin")).expect("runtime");
    std::fs::write(&executable, b"not a java").expect("binary");
    std::fs::write(home.join("release"), "JAVA_VERSION=\"21.0.7\"\n").expect("release");
    assert_eq!(probe_java_major(&executable).expect("release"), 21);

    let eight = root.join("temurin8").join("bin").join("java.exe");
    std::fs::create_dir_all(eight.parent().expect("bin")).expect("runtime");
    std::fs::write(&eight, b"not a java").expect("binary");
    std::fs::write(
        root.join("temurin8").join("release"),
        "JAVA_VERSION=\"1.8.0_432\"\n",
    )
    .expect("release");
    assert_eq!(probe_java_major(&eight).expect("java 8"), 8);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn one_point_eight_and_one_point_twenty_one_need_different_javas() {
    let directory = std::env::temp_dir().join(format!("licht-java-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("directory");
    let java_8 = write_java_stub(&directory, "java8", "1.8.0_51", StubKind::Stderr);
    let java_21 = write_java_stub(&directory, "java21", "21.0.7", StubKind::Stdout);
    let broken = write_java_stub(&directory, "broken", "nope", StubKind::Stderr);
    let failed = write_java_stub(&directory, "failed", "21.0.7", StubKind::Fail);

    let version_8 = parse_version(include_str!("../fixtures/version-1.8.9.json")).expect("1.8.9");
    let version_21 =
        parse_version(include_str!("../fixtures/version-1.21.11.json")).expect("1.21.11");
    let legacy = required_runtime(&version_8).expect("1.8.9 names a runtime");
    let modern = required_runtime(&version_21).expect("1.21.11 names a runtime");

    assert_eq!(probe_java_major(&java_8).expect("java 8"), 8);
    assert_eq!(probe_java_major(&java_21).expect("java 21"), 21);
    assert_eq!(validate_java(&java_8, legacy).expect("match"), 8);
    assert!(matches!(
        validate_java(&java_8, modern),
        Err(CoreError::JavaMajor {
            found: 8,
            required: 21
        })
    ));
    assert!(matches!(
        probe_java_major(&broken),
        Err(CoreError::JavaProbe)
    ));
    assert!(matches!(
        probe_java_major(&failed),
        Err(CoreError::JavaProbe)
    ));

    let candidates = vec![broken, java_21.clone(), failed, java_8.clone()];
    assert_eq!(matching_java(&candidates, legacy).as_ref(), Some(&java_8));
    assert_eq!(matching_java(&candidates, modern).as_ref(), Some(&java_21));
    assert!(matching_java(&[java_8], modern).is_none());
}

enum StubKind {
    Stderr,
    Stdout,
    Fail,
}

fn write_java_stub(directory: &Path, name: &str, version: &str, kind: StubKind) -> PathBuf {
    #[cfg(windows)]
    {
        let path = directory.join(format!("{name}.cmd"));
        let line = match kind {
            StubKind::Stderr => format!(">&2 echo java version \"{version}\"\r\n"),
            StubKind::Stdout => format!("echo java version \"{version}\"\r\n"),
            StubKind::Fail => "exit /b 1\r\n".to_string(),
        };
        std::fs::write(&path, format!("@echo off\r\n{line}")).expect("stub");
        path
    }
    #[cfg(unix)]
    {
        let path = directory.join(name);
        let line = match kind {
            StubKind::Stderr => format!("echo 'java version \"{version}\"' >&2\n"),
            StubKind::Stdout => format!("echo 'java version \"{version}\"'\n"),
            StubKind::Fail => "exit 1\n".to_string(),
        };
        std::fs::write(&path, format!("#!/bin/sh\n{line}")).expect("stub");
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        std::fs::set_permissions(&path, permissions).expect("executable");
        path
    }
}

fn manifest_json(base: &str) -> String {
    format!(
        r#"{{
            "files": {{
                "bin": {{ "type": "directory" }},
                "bin/java.exe": {{
                    "type": "file",
                    "executable": true,
                    "downloads": {{
                        "raw": {{
                            "sha1": "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed",
                            "size": 11,
                            "url": "{base}/java.exe"
                        }}
                    }}
                }},
                "bin/java": {{ "type": "link", "target": "java.exe" }}
            }}
        }}"#
    )
}

struct Server {
    base: String,
    requests: std::sync::Arc<AtomicUsize>,
}

async fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the test server should bind");
    let address = listener
        .local_addr()
        .expect("the test server should have an address");
    let requests = std::sync::Arc::new(AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&requests);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let counted = std::sync::Arc::clone(&counted);
            tokio::spawn(async move {
                let mut incoming = Vec::new();
                let mut buffer = [0_u8; 256];
                loop {
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    if read == 0 {
                        return;
                    }
                    incoming.extend_from_slice(&buffer[..read]);
                    if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                counted.fetch_add(1, Ordering::SeqCst);
                let body = b"hello world";
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if socket.write_all(header.as_bytes()).await.is_err() {
                    return;
                }
                let _ = socket.write_all(body).await;
            });
        }
    });
    Server {
        base: format!("http://{address}"),
        requests,
    }
}
