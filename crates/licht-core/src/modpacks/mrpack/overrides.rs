use std::fs;
use std::io::Cursor;
use std::path::Path;

use serde_json::Value;
use zip::ZipArchive;

use super::PROTECTED_ROOT_NAMES;
use crate::modpacks::archive::read_metadata;
use crate::modpacks::path_safety::safe_instance_path;
use crate::{CoreError, Result};

pub(super) fn read_index(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Value> {
    let file = archive
        .by_name("modrinth.index.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let body = read_metadata(file)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::archive::metadata_zip;

    #[test]
    fn accepts_compressed_index_below_and_at_size_limit() {
        for size in [64 * 1024 * 1024 - 1, 64 * 1024 * 1024] {
            let bytes = metadata_zip(
                "modrinth.index.json",
                r#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#,
                size,
            );
            let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice())).expect("archive");

            let index = read_index(&mut archive).expect("index within the size limit");
            assert_eq!(index["dependencies"]["minecraft"], "1.20.1");
        }
    }

    #[test]
    fn rejects_compressed_index_above_size_limit() {
        let bytes = metadata_zip(
            "modrinth.index.json",
            r#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#,
            64 * 1024 * 1024 + 1,
        );
        assert!(bytes.len() < 128 * 1024, "fixture should remain small");
        let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice())).expect("archive");

        assert!(matches!(
            read_index(&mut archive),
            Err(CoreError::ModpackArchive)
        ));
    }
}
