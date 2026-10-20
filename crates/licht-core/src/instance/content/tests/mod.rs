use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::metadata::{ContentMetadata, write_metadata_cache};
use super::*;
use crate::CoreError;
use crate::hashes::sha1_hex;

mod concurrency;
mod files;
mod fingerprints;
mod progress;
mod providers;

fn cache(name: &str) -> SharedCache {
    let root = std::env::temp_dir().join(format!(
        "licht-content-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("data")).expect("cache root");
    SharedCache::at(root.join("data"))
}

fn instance(cache: &SharedCache) -> PathBuf {
    let root = cache.instance_dir("classic").expect("instance");
    for directory in ["mods", "resourcepacks", "shaderpacks"] {
        fs::create_dir_all(root.join(directory)).expect("content directory");
    }
    root
}

fn seed_metadata(cache: &SharedCache, body: &[u8], title: &str) {
    write_metadata_cache(
        cache,
        &sha1_hex(body),
        &ContentMetadata {
            title: title.to_string(),
            version: Some("1.0".to_string()),
            icon_url: None,
        },
    )
    .expect("cached metadata");
}

fn offline_client() -> Client {
    Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:1").expect("offline proxy"))
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .expect("offline client")
}

fn set_modified(path: &Path, modified: std::time::SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .expect("file")
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("modified time");
}
