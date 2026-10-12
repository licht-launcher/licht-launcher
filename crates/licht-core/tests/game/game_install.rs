use std::collections::BTreeMap;
use std::sync::Arc;

use licht_core::{
    Arch, CoreError, GameInstall, LaunchEnvironment, OsName, SharedCache, install_game,
    parse_install_args,
};
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, mpsc};

const CLIENT: &[u8] = b"client-jar";
const LIBRARY: &[u8] = b"library-jar";
const ASSET: &[u8] = b"asset-bytes";

#[test]
fn install_without_a_version_fails_before_any_download() {
    let error = parse_install_args(&["install".to_string()]).expect_err("version is required");
    assert!(matches!(error, CoreError::LaunchArgs));
}

#[tokio::test]
async fn a_local_version_is_written_and_installed() {
    let directory = std::env::temp_dir().join(format!("licht-game-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("cache root");
    let cache = SharedCache::at(&directory);
    let server = serve().await;
    let base = server.base.clone();
    let asset_hash = sha1_hex(ASSET);
    let index_json = format!(
        r#"{{"objects":{{"icons/icon.png":{{"hash":"{asset_hash}","size":{}}}}}}}"#,
        ASSET.len()
    );
    let version_json = version_json(&base, &sha1_hex(index_json.as_bytes()));
    let version_sha1 = sha1_hex(version_json.as_bytes());
    server.remember("/version", version_json.as_bytes()).await;
    server.remember("/index", index_json.as_bytes()).await;

    let java = directory.join("java.exe");
    std::fs::write(&java, b"java").expect("java file");
    let environment = LaunchEnvironment {
        os: OsName::Linux,
        arch: Arch::X86_64,
        os_version: String::new(),
        features: BTreeMap::new(),
    };
    let (progress, mut incoming) = mpsc::channel(8);
    let installed = install_game(
        &reqwest::Client::new(),
        &cache,
        &cache,
        GameInstall {
            version_id: "demo",
            version_json_url: &format!("{base}/version"),
            version_sha1: &version_sha1,
            environment: &environment,
            asset_base: &base,
            java: Some(&java),
        },
        progress,
    )
    .await
    .expect("install");
    while incoming.recv().await.is_some() {}

    assert_eq!(installed, java);
    assert_eq!(
        std::fs::read(cache.version_json("demo").expect("json path")).expect("json"),
        version_json.as_bytes()
    );
    assert_eq!(
        std::fs::read(cache.version_jar("demo").expect("jar path")).expect("jar"),
        CLIENT
    );

    let error = install_game(
        &reqwest::Client::new(),
        &cache,
        &cache,
        GameInstall {
            version_id: "demo",
            version_json_url: &format!("{base}/version"),
            version_sha1: "abc",
            environment: &environment,
            asset_base: &base,
            java: Some(&java),
        },
        mpsc::channel(8).0,
    )
    .await
    .expect_err("sha1");
    assert!(matches!(error, CoreError::Sha1Mismatch { .. }));

    let _ = std::fs::remove_dir_all(directory);
}

fn version_json(base: &str, index_sha1: &str) -> String {
    let client_sha1 = sha1_hex(CLIENT);
    let library_sha1 = sha1_hex(LIBRARY);
    format!(
        r#"{{"minecraftArguments":"","libraries":[{{"name":"com.example:demo:1.0","downloads":{{"artifact":{{"path":"com/example/demo/1.0/demo-1.0.jar","sha1":"{library_sha1}","size":1,"url":"{base}/library"}}}}}}],"assetIndex":{{"id":"test","sha1":"{index_sha1}","size":1,"totalSize":1,"url":"{base}/index"}},"mainClass":"a.B","downloads":{{"client":{{"sha1":"{client_sha1}","size":{client_size},"url":"{base}/client"}}}}}}"#,
        client_size = CLIENT.len()
    )
}

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

struct Server {
    base: String,
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl Server {
    async fn remember(&self, path: &str, body: &[u8]) {
        self.files
            .lock()
            .await
            .insert(path.to_string(), body.to_vec());
    }
}

async fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the test server should bind");
    let address = listener
        .local_addr()
        .expect("the test server should have an address");
    let files = Arc::new(Mutex::new(BTreeMap::from([
        ("/client".to_string(), CLIENT.to_vec()),
        ("/library".to_string(), LIBRARY.to_vec()),
    ])));
    let base = format!("http://{address}");
    let files_for_server = Arc::clone(&files);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let files = Arc::clone(&files);
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
                let payload = body.unwrap_or_default();
                let status = if payload.is_empty() { 500 } else { 200 };
                let header = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        base,
        files: files_for_server,
    }
}

fn request_path(request: &[u8]) -> String {
    let text = String::from_utf8_lossy(request);
    text.split_whitespace().nth(1).unwrap_or("/").to_string()
}
