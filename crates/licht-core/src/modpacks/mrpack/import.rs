use std::io::Cursor;
use std::path::Path;

use zip::ZipArchive;

use super::overrides::{extract_prefixed, read_index};
use super::storage::StoredModpack;
use crate::instance::InstanceEntry;
use crate::{CoreError, Result};

pub(crate) async fn import_mrpack_into(
    database: &crate::DatabaseClient,
    instances_dir: &Path,
    entry: &InstanceEntry,
    bytes: &[u8],
) -> Result<InstanceEntry> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let index = read_index(&mut archive)?;
    let dir = instances_dir.join(&entry.folder);
    let stored = StoredModpack::parse(&index, &dir)?;
    let version = index
        .pointer("/dependencies/minecraft")
        .and_then(serde_json::Value::as_str)
        .ok_or(CoreError::ModpackArchive)?;
    let updated = crate::update_instance_version(instances_dir, &entry.folder, version)?;
    extract_prefixed(&mut archive, &dir, "overrides/")?;
    extract_prefixed(&mut archive, &dir, "client-overrides/")?;
    database
        .replace_modpack(instances_dir, &entry.folder, stored)
        .await?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GlobalLaunch, InstanceSource, ModpackSource, create_pending_instance};
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    #[tokio::test]
    async fn imports_overrides_and_minecraft_version() {
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
        let pending = pending(&instances);
        let service = crate::DatabaseService::start(crate::SharedCache::at(&root))
            .await
            .expect("database");
        let entry = import_mrpack_into(&service.client(), &instances, &pending, &bytes)
            .await
            .expect("import");
        assert_eq!(entry.folder, pending.folder);
        assert_eq!(entry.version_id.as_deref(), Some("1.20.1"));
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("options.txt")).unwrap(),
            "fov:90"
        );
        assert_eq!(
            fs::read_to_string(instances.join(&entry.folder).join("client.txt")).unwrap(),
            "ok"
        );
        assert!(
            !instances
                .join(&entry.folder)
                .join(crate::MODPACK_INDEX_FILE)
                .exists()
        );
        assert_eq!(
            service
                .client()
                .modpack_dependencies(&instances, &entry.folder)
                .await
                .expect("dependencies"),
            [("minecraft".to_string(), "1.20.1".to_string())]
        );
        service.shutdown().await.expect("shutdown");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn rejects_overrides_that_target_launcher_files_or_windows_devices() {
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
            let pending = pending(&instances);
            let service = crate::DatabaseService::start(crate::SharedCache::at(&root))
                .await
                .expect("database");
            let error = import_mrpack_into(&service.client(), &instances, &pending, &bytes)
                .await
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
            service.shutdown().await.expect("shutdown");
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

    fn pending(instances: &Path) -> InstanceEntry {
        create_pending_instance(
            instances,
            "Cobble",
            InstanceSource::Modpack {
                source: ModpackSource::Modrinth,
                project_id: "project".into(),
                version_id: "version".into(),
                icon_url: None,
            },
            &launch_template(),
        )
        .expect("pending instance")
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
