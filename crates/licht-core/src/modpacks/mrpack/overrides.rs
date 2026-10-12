use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use serde_json::Value;
use zip::ZipArchive;

use super::PROTECTED_ROOT_NAMES;
use crate::modpacks::path_safety::safe_instance_path;
use crate::{CoreError, Result};

pub(super) fn read_index(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Value> {
    let mut file = archive
        .by_name("modrinth.index.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let mut body = String::new();
    file.read_to_string(&mut body)
        .map_err(|_| CoreError::ModpackArchive)?;
    serde_json::from_str(&body).map_err(CoreError::LoaderJson)
}

pub(super) fn extract_prefixed(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    instance_dir: &Path,
    prefix: &str,
) -> Result<()> {
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let name = file.name().replace('\\', "/");
        let Some(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        // Rejects absolute paths and `..` that leave the archive. `C:foo` still
        // looks like a normal name on Unix, so `safe_instance_path` checks it too.
        if file.enclosed_name().is_none() {
            return Err(CoreError::ModpackArchive);
        }
        let destination = safe_instance_path(instance_dir, relative, &PROTECTED_ROOT_NAMES)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}
