use std::io::{Cursor, Read};

use serde_json::Value;
use zip::ZipArchive;

use crate::{CoreError, Result};

#[derive(Debug)]
pub(super) struct Manifest {
    pub(super) game_version: String,
    pub(super) files: Vec<ManifestFile>,
    pub(super) overrides_dir: String,
}

#[derive(Debug)]
pub(super) struct ManifestFile {
    pub(super) project_id: u64,
    pub(super) file_id: u64,
    pub(super) required: bool,
}

pub(super) fn manifest_from_zip(bytes: &[u8]) -> Result<Manifest> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let mut file = archive
        .by_name("manifest.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let mut body = String::new();
    file.read_to_string(&mut body)
        .map_err(|_| CoreError::ModpackArchive)?;
    parse_manifest(&body)
}

fn parse_manifest(body: &str) -> Result<Manifest> {
    let value: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let game_version = value
        .pointer("/minecraft/version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .ok_or(CoreError::ModpackArchive)?
        .to_string();
    let files = match value.get("files") {
        None => Vec::new(),
        Some(Value::Array(items)) => parse_file_entries(items)?,
        Some(_) => return Err(CoreError::ModpackArchive),
    };
    let overrides_dir = value
        .get("overrides")
        .and_then(Value::as_str)
        .unwrap_or("overrides")
        .to_string();
    Ok(Manifest {
        game_version,
        files,
        overrides_dir,
    })
}

fn parse_file_entries(items: &[Value]) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();
    for item in items {
        let project_id = item
            .get("projectID")
            .and_then(Value::as_u64)
            .ok_or(CoreError::ModpackArchive)?;
        let file_id = item
            .get("fileID")
            .and_then(Value::as_u64)
            .ok_or(CoreError::ModpackArchive)?;
        let required = item
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        files.push(ManifestFile {
            project_id,
            file_id,
            required,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_files_and_defaults_required() {
        let manifest = parse_manifest(
            r#"{"minecraft":{"version":"1.20.1"},"files":[{"projectID":1,"fileID":2}]}"#,
        )
        .expect("manifest");
        assert_eq!(manifest.game_version, "1.20.1");
        assert_eq!(manifest.overrides_dir, "overrides");
        assert!(manifest.files[0].required);
        assert_eq!(manifest.files[0].file_id, 2);
    }

    #[test]
    fn rejects_a_manifest_without_a_minecraft_version() {
        let error = parse_manifest(r#"{"files":[]}"#).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
    }
}
