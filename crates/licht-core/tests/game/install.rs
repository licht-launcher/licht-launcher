use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use licht_core::{
    Arch, Artifact, AssetIndex, AssetIndexFile, AssetObject, CoreError, Download, DownloadProgress,
    GameArguments, InstallPlan, LaunchEnvironment, Library, LibraryDownloads, OsName, OsRule, Rule,
    RuleAction, SharedCache, Version, VersionDownloads, install_version,
};
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, mpsc};

const CLIENT: &[u8] = b"client-jar";
const LIBRARY: &[u8] = b"library-jar";
const ASSET: &[u8] = b"asset-bytes";
const NATIVE: &[u8] = b"native-64";

#[tokio::test]
async fn installing_twice_downloads_only_the_first_time() {
    let directory = temporary_directory("install");
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let asset_hash = sha1_hex(ASSET);
    let index_json = index_json(&asset_hash);
    let version = version(&server.base, &sha1_hex(index_json.as_bytes()));
    let assets = assets(&asset_hash);
    let environment = environment();

    let first = install(
        &cache,
        &server,
        &version,
        &index_json,
        &assets,
        &environment,
    )
    .await;
    first.expect("the first install should succeed");
    let requests_after_first = server.requests.load(Ordering::SeqCst);
    assert!(requests_after_first >= 3);

    assert_eq!(
        std::fs::read(cache.version_jar("1.21.11").unwrap()).unwrap(),
        CLIENT
    );
    assert_eq!(
        std::fs::read(cache.library("com/example/demo/1.0/demo-1.0.jar").unwrap(),).unwrap(),
        LIBRARY,
    );
    assert_eq!(
        std::fs::read(cache.asset_object(&asset_hash).unwrap()).unwrap(),
        ASSET,
    );
    assert_eq!(
        std::fs::read(cache.asset_index("test").unwrap()).unwrap(),
        index_json.as_bytes(),
    );
    assert!(
        !cache
            .library("com/example/windows/1.0/windows-1.0.jar")
            .unwrap()
            .exists()
    );

    install(
        &cache,
        &server,
        &version,
        &index_json,
        &assets,
        &environment,
    )
    .await
    .expect("the second install should skip every file");
    assert_eq!(server.requests.load(Ordering::SeqCst), requests_after_first);

    std::fs::write(cache.version_jar("1.21.11").unwrap(), b"corrupt").unwrap();
    install(
        &cache,
        &server,
        &version,
        &index_json,
        &assets,
        &environment,
    )
    .await
    .expect("a corrupt file should be downloaded again");
    assert!(server.requests.load(Ordering::SeqCst) > requests_after_first);
    assert_eq!(
        std::fs::read(cache.version_jar("1.21.11").unwrap()).unwrap(),
        CLIENT
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[tokio::test]
async fn an_index_with_the_wrong_sha1_downloads_nothing() {
    let directory = temporary_directory("bad-index");
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let asset_hash = sha1_hex(ASSET);
    let version = version(&server.base, &"a".repeat(40));
    let assets = assets(&asset_hash);

    let error = install(
        &cache,
        &server,
        &version,
        "{\"objects\":{}}",
        &assets,
        &environment(),
    )
    .await
    .expect_err("the index hash should be rejected");

    assert!(matches!(error, CoreError::Sha1Mismatch { .. }));
    assert_eq!(server.requests.load(Ordering::SeqCst), 0);
    let _ = std::fs::remove_dir_all(directory);
}

#[tokio::test]
async fn an_architecture_placeholder_downloads_only_that_native() {
    let directory = temporary_directory("native-arch");
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let asset_hash = sha1_hex(ASSET);
    let index_json = index_json(&asset_hash);
    let mut version = version(&server.base, &sha1_hex(index_json.as_bytes()));
    version.libraries.retain(|library| library.rules.is_none());
    version.libraries.push(Library {
        name: "tv.twitch:twitch-platform:6.5".to_string(),
        downloads: Some(LibraryDownloads {
            artifact: None,
            classifiers: Some(BTreeMap::from([
                (
                    "natives-windows-64".to_string(),
                    Artifact {
                        path: "tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-64.jar".to_string(),
                        sha1: sha1_hex(NATIVE),
                        size: NATIVE.len() as u64,
                        url: format!("{}/native64", server.base),
                    },
                ),
                (
                    "natives-windows-32".to_string(),
                    Artifact {
                        path: "tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-32.jar".to_string(),
                        sha1: sha1_hex(b"native-32"),
                        size: 8,
                        url: format!("{}/native32", server.base),
                    },
                ),
            ])),
        }),
        natives: Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows-${arch}".to_string(),
        )])),
        rules: None,
        extract: None,
    });

    install(
        &cache,
        &server,
        &version,
        &index_json,
        &assets(&asset_hash),
        &LaunchEnvironment {
            os: OsName::Windows,
            arch: Arch::X86_64,
            os_version: String::new(),
            features: BTreeMap::new(),
        },
    )
    .await
    .expect("the 64-bit native should download");

    assert_eq!(
        std::fs::read(
            cache
                .library(
                    "tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-64.jar",
                )
                .unwrap(),
        )
        .unwrap(),
        NATIVE,
    );
    assert!(
        !cache
            .library("tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-32.jar")
            .unwrap()
            .exists()
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[tokio::test]
async fn only_the_running_platform_native_is_stored() {
    let directory = temporary_directory("natives-platform");
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let asset_hash = sha1_hex(ASSET);
    let index_json = index_json(&asset_hash);
    let mut version = version(&server.base, &sha1_hex(index_json.as_bytes()));
    version
        .libraries
        .retain(|library| library.name != "com.example:windows:1.0");
    version.libraries.push(native_jar(
        "org.lwjgl:lwjgl:3.3.3:natives-windows",
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar",
        "windows",
        &server.base,
    ));
    version.libraries.push(native_jar(
        "org.lwjgl:lwjgl:3.3.3:natives-windows-x86",
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows-x86.jar",
        "windows",
        &server.base,
    ));
    version.libraries.push(native_jar(
        "org.lwjgl:lwjgl:3.3.3:natives-windows-arm64",
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows-arm64.jar",
        "windows",
        &server.base,
    ));
    version.libraries.push(native_jar(
        "org.lwjgl:lwjgl:3.3.3:natives-linux",
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar",
        "linux",
        &server.base,
    ));
    version.libraries.push(legacy_native_jars(&server.base));
    let environment = LaunchEnvironment {
        os: OsName::Windows,
        arch: Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    let assets = assets(&asset_hash);
    install(
        &cache,
        &server,
        &version,
        &index_json,
        &assets,
        &environment,
    )
    .await
    .expect("install");

    assert!(
        cache
            .library("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar")
            .expect("x64 path")
            .is_file()
    );
    assert!(
        !cache
            .library("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows-x86.jar")
            .expect("x86 path")
            .exists()
    );
    assert!(
        !cache
            .library("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows-arm64.jar")
            .expect("arm path")
            .exists()
    );
    assert!(
        !cache
            .library("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
            .expect("linux path")
            .exists()
    );
    assert!(
        cache
            .library("org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows-64.jar")
            .expect("legacy 64 path")
            .is_file()
    );
    assert!(
        !cache
            .library("org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows-32.jar")
            .expect("legacy 32 path")
            .exists()
    );
    let _ = std::fs::remove_dir_all(directory);
}

fn native_jar(name: &str, path: &str, os_name: &str, base: &str) -> Library {
    library(
        name,
        &format!("{base}/library"),
        path,
        &sha1_hex(LIBRARY),
        Some(vec![Rule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some(os_name.to_string()),
                arch: None,
                version: None,
            }),
            features: None,
        }]),
    )
}

fn legacy_native_jars(base: &str) -> Library {
    let artifact = |path: &str| Artifact {
        path: path.to_string(),
        sha1: sha1_hex(LIBRARY),
        size: LIBRARY.len() as u64,
        url: format!("{base}/library"),
    };
    Library {
        name: "org.lwjgl.lwjgl:lwjgl-platform:2.9.4".to_string(),
        downloads: Some(LibraryDownloads {
            artifact: None,
            classifiers: Some(BTreeMap::from([
                (
                    "natives-windows-64".to_string(),
                    artifact(
                        "org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows-64.jar",
                    ),
                ),
                (
                    "natives-windows-32".to_string(),
                    artifact(
                        "org/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows-32.jar",
                    ),
                ),
            ])),
        }),
        natives: Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows-${arch}".to_string(),
        )])),
        rules: None,
        extract: None,
    }
}

struct Server {
    base: String,
    requests: Arc<AtomicUsize>,
}

async fn install(
    cache: &SharedCache,
    server: &Server,
    version: &Version,
    index_json: &str,
    assets: &AssetIndexFile,
    environment: &LaunchEnvironment,
) -> licht_core::Result<()> {
    let (progress, mut incoming) = mpsc::channel::<DownloadProgress>(8);
    let result = install_version(
        &reqwest::Client::new(),
        cache,
        cache,
        InstallPlan {
            version_id: "1.21.11",
            version,
            asset_index_json: index_json,
            assets,
            environment,
            asset_base: &server.base,
        },
        progress,
    )
    .await;
    while incoming.recv().await.is_some() {}
    result
}

fn version(base: &str, index_sha1: &str) -> Version {
    Version {
        arguments: GameArguments::Legacy(String::new()),
        libraries: vec![
            library(
                "com.example:demo:1.0",
                &format!("{base}/library"),
                "com/example/demo/1.0/demo-1.0.jar",
                &sha1_hex(LIBRARY),
                None,
            ),
            library(
                "com.example:windows:1.0",
                &format!("{base}/windows"),
                "com/example/windows/1.0/windows-1.0.jar",
                &sha1_hex(b"windows"),
                Some(vec![Rule {
                    action: RuleAction::Allow,
                    os: Some(OsRule {
                        name: Some("windows".to_string()),
                        arch: None,
                        version: None,
                    }),
                    features: None,
                }]),
            ),
        ],
        asset_index: AssetIndex {
            id: "test".to_string(),
            sha1: index_sha1.to_string(),
            size: 1,
            total_size: 1,
            url: format!("{base}/index"),
        },
        main_class: "a.B".to_string(),
        downloads: VersionDownloads {
            client: Download {
                sha1: sha1_hex(CLIENT),
                size: CLIENT.len() as u64,
                url: format!("{base}/client"),
            },
            server: None,
        },
        java_version: None,
    }
}

fn library(name: &str, url: &str, path: &str, sha1: &str, rules: Option<Vec<Rule>>) -> Library {
    Library {
        name: name.to_string(),
        downloads: Some(LibraryDownloads {
            artifact: Some(Artifact {
                path: path.to_string(),
                sha1: sha1.to_string(),
                size: 1,
                url: url.to_string(),
            }),
            classifiers: None,
        }),
        natives: None,
        rules,
        extract: None,
    }
}

fn assets(hash: &str) -> AssetIndexFile {
    AssetIndexFile {
        objects: BTreeMap::from([(
            "icons/icon.png".to_string(),
            AssetObject {
                hash: hash.to_string(),
                size: ASSET.len() as u64,
            },
        )]),
        virtual_assets: false,
        map_to_resources: false,
    }
}

fn index_json(asset_hash: &str) -> String {
    format!(
        r#"{{"objects":{{"icons/icon.png":{{"hash":"{asset_hash}","size":{}}}}}}}"#,
        ASSET.len()
    )
}

fn environment() -> LaunchEnvironment {
    LaunchEnvironment {
        os: OsName::Linux,
        arch: Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn temporary_directory(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("licht-install-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    directory
}

async fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the test server should bind");
    let address = listener
        .local_addr()
        .expect("the test server should have an address");
    let requests = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&requests);
    let files = Arc::new(Mutex::new(BTreeMap::from([
        ("/client".to_string(), CLIENT.to_vec()),
        ("/library".to_string(), LIBRARY.to_vec()),
        ("/native64".to_string(), NATIVE.to_vec()),
    ])));

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let files = Arc::clone(&files);
            let counted = Arc::clone(&counted);
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
                let path = request_path(&incoming);
                let body = {
                    let mut files = files.lock().await;
                    if let Some(hash) = path.strip_prefix('/')
                        && let Some((prefix, hash)) = hash.split_once('/')
                        && hash.starts_with(prefix)
                    {
                        files.entry(path.clone()).or_insert_with(|| ASSET.to_vec());
                    }
                    files.get(&path).cloned()
                };
                let (status, payload) = match body {
                    Some(payload) => (200, payload),
                    None => (500, Vec::new()),
                };
                let reason = if status == 200 { "OK" } else { "Error" };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                if socket.write_all(header.as_bytes()).await.is_err() {
                    return;
                }
                let _ = socket.write_all(&payload).await;
            });
        }
    });

    Server {
        base: format!("http://{address}"),
        requests,
    }
}

fn request_path(request: &[u8]) -> String {
    let text = String::from_utf8_lossy(request);
    text.split_whitespace().nth(1).unwrap_or("/").to_string()
}
