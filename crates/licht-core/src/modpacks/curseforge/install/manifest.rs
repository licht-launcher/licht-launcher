use std::io::Cursor;

use serde_json::Value;
use zip::ZipArchive;

use crate::modpacks::archive::read_metadata;
use crate::{CoreError, Result};

#[derive(Debug)]
pub(super) struct Manifest {
    pub(super) game_version: String,
    pub(super) files: Vec<ManifestFile>,
    pub(super) overrides_dir: String,
    pub(super) loader: Option<(crate::LoaderPlatform, String)>,
}

#[derive(Debug)]
pub(super) struct ManifestFile {
    pub(super) project_id: u64,
    pub(super) file_id: u64,
    pub(super) required: bool,
}

pub(super) fn manifest_from_zip(bytes: &[u8]) -> Result<Manifest> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let file = archive
        .by_name("manifest.json")
        .map_err(|_| CoreError::ModpackArchive)?;
    let body = read_metadata(file)?;
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
        loader: value
            .pointer("/minecraft/modLoaders")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("primary").and_then(Value::as_bool) == Some(true))
                    .or_else(|| items.first())
            })
            .map(|item| {
                let id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or(CoreError::ModpackArchive)?;
                let (family, version) =
                    id.split_once('-').ok_or(CoreError::LoaderVersionMissing)?;
                let platform = crate::LoaderPlatform::parse(family)
                    .filter(|platform| *platform != crate::LoaderPlatform::Vanilla)
                    .ok_or(CoreError::LoaderVersionMissing)?;
                if version.is_empty() {
                    return Err(CoreError::LoaderVersionMissing);
                }
                Ok((platform, version.to_string()))
            })
            .transpose()?,
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
    use crate::modpacks::archive::metadata_zip;

    #[test]
    fn accepts_compressed_manifest_below_and_at_size_limit() {
        for size in [64 * 1024 * 1024 - 1, 64 * 1024 * 1024] {
            let bytes = metadata_zip(
                "manifest.json",
                r#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
                size,
            );

            let manifest = manifest_from_zip(&bytes).expect("manifest within the size limit");
            assert_eq!(manifest.game_version, "1.20.1");
            assert!(manifest.files.is_empty());
        }
    }

    #[test]
    fn rejects_compressed_manifest_above_size_limit() {
        let bytes = metadata_zip(
            "manifest.json",
            r#"{"minecraft":{"version":"1.20.1"},"files":[]}"#,
            64 * 1024 * 1024 + 1,
        );
        assert!(bytes.len() < 128 * 1024, "fixture should remain small");

        assert!(matches!(
            manifest_from_zip(&bytes),
            Err(CoreError::ModpackArchive)
        ));
    }

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

    #[test]
    fn recognizes_each_supported_primary_loader_without_assuming_vanilla() {
        for (platform, id) in [
            (crate::LoaderPlatform::Fabric, "fabric-0.16.10"),
            (crate::LoaderPlatform::Quilt, "quilt-0.26.0"),
            (crate::LoaderPlatform::Forge, "forge-51.0.0"),
            (crate::LoaderPlatform::NeoForge, "neoforge-21.1.80"),
        ] {
            let body = serde_json::json!({ "minecraft": { "version": "1.21.1", "modLoaders": [
                { "id": "forge-unused", "primary": false }, { "id": id, "primary": true }
            ] } });
            let manifest = parse_manifest(&body.to_string()).expect("manifest");
            assert_eq!(
                manifest.loader.as_ref().map(|(selected, _)| *selected),
                Some(platform)
            );
            assert_eq!(
                manifest.loader.expect("loader").1,
                id.split_once('-').expect("build").1
            );
        }
        let invalid = r#"{"minecraft":{"version":"1.21.1","modLoaders":[{"id":"unsupported-1","primary":true}]}}"#;
        assert!(matches!(
            parse_manifest(invalid),
            Err(CoreError::LoaderVersionMissing)
        ));
    }
}
