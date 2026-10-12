use std::fs;
use std::io::Cursor;
use std::path::Path;

use zip::ZipArchive;

use super::MODPACK_INDEX_FILE;
use super::overrides::{extract_prefixed, read_index};
use crate::instance::{InstanceEntry, create_instance};
use crate::settings::GlobalLaunch;
use crate::{CoreError, Result};

/// Extracts a Modrinth `.mrpack` (zip) into a new instance folder.
///
/// Applies `overrides/` and `client-overrides/` into the instance root and
/// records the full index in [`MODPACK_INDEX_FILE`] for the download pass.
pub fn import_mrpack_bytes(
    instances_dir: &Path,
    name: &str,
    bytes: &[u8],
    template: &GlobalLaunch,
) -> Result<InstanceEntry> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| CoreError::ModpackArchive)?;
    let index = read_index(&mut archive)?;
    let game_version = index
        .pointer("/dependencies/minecraft")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let entry = create_instance(instances_dir, name, &game_version, template)?;
    let instance_dir = instances_dir.join(&entry.folder);
    extract_prefixed(&mut archive, &instance_dir, "overrides/")?;
    extract_prefixed(&mut archive, &instance_dir, "client-overrides/")?;
    let meta_path = instance_dir.join(MODPACK_INDEX_FILE);
    fs::write(
        meta_path,
        serde_json::to_string_pretty(&index).map_err(CoreError::LoaderJson)?,
    )?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    #[test]
    fn imports_overrides_and_minecraft_version() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("modrinth.index.json", options).unwrap();
            zip.write_all(br#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#)
                .unwrap();
            zip.start_file("overrides/options.txt", options).unwrap();
            zip.write_all(b"fov:90").unwrap();
            zip.start_file("client-overrides/client.txt", options)
                .unwrap();
            zip.write_all(b"ok").unwrap();
            zip.finish().unwrap();
        }
        let bytes = buffer.into_inner();
        let root = std::env::temp_dir().join(format!("licht-mrpack-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let instances = root.join("instances");
        fs::create_dir_all(&instances).unwrap();
        let entry =
            import_mrpack_bytes(&instances, "Cobble", &bytes, &launch_template()).expect("import");
        assert_eq!(entry.version_id, "1.20.1");
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("options.txt")).unwrap(),
            "fov:90"
        );
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("client.txt")).unwrap(),
            "ok"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_overrides_that_target_launcher_files_or_windows_devices() {
        let names = [
            "overrides/instance.json",
            "overrides/licht-modpack.json",
            "overrides/C:evil.txt",
            "overrides/CON",
            "overrides/mods/CON.txt",
            "client-overrides/../../outside.txt",
        ];
        for name in names {
            let bytes = zip_with(&[
                (
                    "modrinth.index.json",
                    br#"{"dependencies":{"minecraft":"1.20.1"},"files":[]}"#,
                ),
                (name, b"pwned"),
            ]);
            let root = temp_dir("override");
            let instances = root.join("instances");
            fs::create_dir_all(&instances).unwrap();
            let error = import_mrpack_bytes(&instances, "Cobble", &bytes, &launch_template())
                .expect_err(name);
            assert!(matches!(error, CoreError::ModpackArchive), "{name}");
            let instance_dir = fs::read_dir(&instances)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let profile = fs::read_to_string(instance_dir.join("instance.json")).unwrap();
            assert!(!profile.contains("pwned"), "{name}");
            assert!(!instance_dir.join("C:evil.txt").exists(), "{name}");
            assert!(!instance_dir.join("CON").exists(), "{name}");
            let _ = fs::remove_dir_all(&root);
        }
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

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-mrpack-{label}-{}-{stamp}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
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
}
