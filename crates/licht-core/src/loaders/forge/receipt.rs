use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::artifacts::{failure, valid_sha1};
use crate::hashes::{file_matches_sha1, sha1_hex};
use crate::{Result, SharedCache};

const RECEIPT: &str = "licht-loader-install.json";

#[derive(Serialize, Deserialize)]
struct Receipt {
    schema: u32,
    profile_sha1: String,
    client_sha1: String,
    files: BTreeMap<String, String>,
}

fn path(cache: &SharedCache, id: &str) -> Result<PathBuf> {
    Ok(cache
        .version_json(id)?
        .parent()
        .ok_or(crate::CoreError::CachePath)?
        .join(RECEIPT))
}

pub(super) fn ready(cache: &SharedCache, id: &str) -> bool {
    match check(cache, id) {
        Ok(ready) => ready,
        Err(error) => {
            tracing::warn!(error = %error, "Loader installation receipt is invalid");
            false
        }
    }
}

fn check(cache: &SharedCache, id: &str) -> Result<bool> {
    let receipt_path = path(cache, id)?;
    if !receipt_path.is_file() {
        return Ok(false);
    }
    let receipt: Receipt =
        serde_json::from_slice(&fs::read(receipt_path)?).map_err(crate::CoreError::LoaderJson)?;
    if receipt.schema != 1
        || receipt.files.is_empty()
        || !file_matches_sha1(&cache.version_json(id)?, &receipt.profile_sha1)?
        || !file_matches_sha1(&cache.version_jar(id)?, &receipt.client_sha1)?
    {
        return Ok(false);
    }
    for (relative, hash) in receipt.files {
        if !valid_sha1(&hash) || !file_matches_sha1(&cache.library(&relative)?, &hash)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn record(cache: &SharedCache, id: &str, files: &[PathBuf]) -> Result<()> {
    let mut verified = BTreeMap::new();
    let library_root = cache.root().join("libraries");
    for file in files {
        let relative = file
            .strip_prefix(&library_root)
            .map_err(|_| failure("processor output is outside the library cache"))?;
        let relative = relative
            .to_str()
            .ok_or_else(|| failure("library path is not UTF-8"))?
            .replace('\\', "/");
        cache.library(&relative)?;
        verified.insert(relative, sha1_hex(&fs::read(file)?));
    }
    let profile_sha1 = sha1_hex(&fs::read(cache.version_json(id)?)?);
    let client_sha1 = sha1_hex(&fs::read(cache.version_jar(id)?)?);
    let receipt = Receipt {
        schema: 1,
        profile_sha1,
        client_sha1,
        files: verified,
    };
    let destination = path(cache, id)?;
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(crate::CoreError::LoaderJson)?;
    atomic_write(&destination, &bytes)
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(path.parent().ok_or(crate::CoreError::CachePath)?)?;
    let temporary = path.with_extension("part");
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}
