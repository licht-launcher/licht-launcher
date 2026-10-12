//! Modpack discovery (Modrinth / CurseForge) and `.mrpack` import.

mod curseforge;
mod file_hashes;
mod modrinth;
mod mrpack;
mod path_safety;
mod secure_download;
mod types;

pub use curseforge::CURSEFORGE_API_KEY_ENV;
pub use mrpack::{
    MODPACK_INDEX_FILE, apply_mrpack_loader, import_mrpack_bytes, install_mrpack_files,
    mrpack_download_tasks,
};
pub use types::{ModpackListItem, ModpackSource, ModpackVersionItem};

use reqwest::Client;

use crate::instance::InstanceEntry;
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result, SharedCache};
use path_safety::safe_instance_path;
use std::path::Path;

/// Searches Modrinth always; CurseForge when `LICHT_CURSEFORGE_API_KEY` is set.
pub async fn search_modpacks(client: &Client, query: &str) -> Result<Vec<ModpackListItem>> {
    let mut items = modrinth::search_modrinth(client, query).await?;
    match curseforge::search_curseforge(client, query).await {
        Ok(mut curse) => items.append(&mut curse),
        Err(CoreError::CurseforgeKeyMissing) => {}
        Err(error) => return Err(error),
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.downloads));
    Ok(items)
}

pub async fn list_modpack_versions(
    client: &Client,
    source: ModpackSource,
    project_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    match source {
        ModpackSource::Modrinth => modrinth::list_modrinth_versions(client, project_id).await,
        ModpackSource::Curseforge => curseforge::list_curseforge_versions(client, project_id).await,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_instance_from_modpack(
    client: &Client,
    cache: &SharedCache,
    instances_dir: &Path,
    name: &str,
    source: ModpackSource,
    project_id: &str,
    version_id: &str,
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let (_file_name, bytes) = match source {
        ModpackSource::Modrinth => {
            modrinth::download_modrinth_mrpack(client, project_id, version_id).await?
        }
        ModpackSource::Curseforge => {
            curseforge::download_curseforge_pack(client, project_id, version_id).await?
        }
    };
    match source {
        ModpackSource::Modrinth => {
            let entry = import_mrpack_bytes(instances_dir, name, &bytes, template)?;
            apply_mrpack_loader(client, cache, instances_dir, &entry).await
        }
        ModpackSource::Curseforge => {
            // Curse zip layouts vary; store the archive beside a vanilla-backed instance
            // and extract overlays when `manifest.json` / overrides exist.
            // Full Curse file resolution is out of scope for this pass.
            import_curse_zip(instances_dir, name, &bytes, template)
        }
    }
}

fn import_curse_zip(
    instances_dir: &Path,
    name: &str,
    bytes: &[u8],
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| CoreError::ModpackArchive)?;
    let mut game_version = "1.20.1".to_string();
    if let Ok(mut manifest) = archive.by_name("manifest.json") {
        let mut body = String::new();
        manifest
            .read_to_string(&mut body)
            .map_err(|_| CoreError::ModpackArchive)?;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
            && let Some(id) = value
                .pointer("/minecraft/version")
                .and_then(serde_json::Value::as_str)
        {
            game_version = id.to_string();
        }
    }
    let entry = crate::instance::create_instance(instances_dir, name, &game_version, template)?;
    let instance_dir = instances_dir.join(&entry.folder);
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let name = file.name().replace('\\', "/");
        let Some(relative) = name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("override/"))
        else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        // Same checks as `.mrpack` overrides: `enclosed_name` rejects absolute
        // paths and `..`, `safe_instance_path` the Windows-specific cases.
        if file.enclosed_name().is_none() {
            return Err(CoreError::ModpackArchive);
        }
        let destination = safe_instance_path(
            &instance_dir,
            relative,
            &["instance.json", MODPACK_INDEX_FILE],
        )?;
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    fn curse_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for (name, bytes) in files {
                zip.start_file(*name, options).unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn launch_template() -> GlobalLaunch {
        GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: Vec::new(),
            fullscreen: false,
            width: None,
            height: None,
        }
    }

    fn temp_instances(label: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-curse-{label}-{}-{stamp}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn curse_import_extracts_overrides_and_reads_the_game_version() {
        let bytes = curse_zip(&[
            ("manifest.json", br#"{"minecraft":{"version":"1.19.2"}}"#),
            ("overrides/config/a.toml", b"x=1"),
            ("override/options.txt", b"fov:90"),
        ]);
        let root = temp_instances("ok");
        let entry = import_curse_zip(&root, "Curse", &bytes, &launch_template()).expect("import");
        assert_eq!(entry.version_id, "1.19.2");
        let dir = root.join(&entry.folder);
        assert_eq!(std::fs::read(dir.join("config/a.toml")).unwrap(), b"x=1");
        assert_eq!(std::fs::read(dir.join("options.txt")).unwrap(), b"fov:90");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn curse_import_rejects_unsafe_override_paths() {
        for name in [
            "overrides/instance.json",
            "overrides/licht-modpack.json",
            "overrides/C:evil.txt",
            "overrides/CON",
            "overrides/mods/CON.txt",
            "overrides/../../outside.txt",
            "overrides/..\\..\\outside.txt",
            "override/instance.json",
        ] {
            let bytes = curse_zip(&[("manifest.json", br#"{}"#), (name, b"pwned")]);
            let root = temp_instances("bad");
            let error =
                import_curse_zip(&root, "Curse", &bytes, &launch_template()).expect_err(name);
            assert!(matches!(error, CoreError::ModpackArchive), "{name}");
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}
