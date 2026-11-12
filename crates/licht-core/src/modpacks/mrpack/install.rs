use std::fs;
use std::path::Path;

use super::MODPACK_INDEX_FILE;
use super::index::MRPACK_DOWNLOAD_HOSTS;
use super::storage::StoredModpack;
use crate::modpacks::file_hashes::{ensure_sha512, prepare_download};
use crate::modpacks::path_safety::instance_key;
use crate::modpacks::secure_download::allowlisted_download_client;
use crate::modpacks::{CURSEFORGE_INDEX_FILE, remove_legacy_index};
use crate::transfer::verification::VerificationProgress;
use crate::{CoreError, Result, SharedCache};
use reqwest::Client;
use serde_json::Value;

/// Downloads every client-required file from the stored Modrinth index into the instance.
///
/// No-op when no files are stored (vanilla / CurseForge without mrpack files).
/// A legacy [`MODPACK_INDEX_FILE`] is imported into SQLite first and then
/// removed, unless the instance has CurseForge files. A redirect is followed
/// only when that hop is HTTPS and the host is `cdn.modrinth.com`.
pub async fn install_mrpack_files(
    database: &crate::DatabaseClient,
    game: &SharedCache,
    settings: &crate::Database,
    instance_dir: &Path,
    progress: impl Into<crate::InstallationContext>,
) -> Result<()> {
    install_files(
        database,
        game,
        settings,
        instance_dir,
        progress.into(),
        &mrpack_download_client()?,
    )
    .await
}

async fn install_files(
    database: &crate::DatabaseClient,
    game: &SharedCache,
    settings: &crate::Database,
    instance_dir: &Path,
    progress: crate::InstallationContext,
    client: &Client,
) -> Result<()> {
    // Pack indexes can contain stale sizes; downloaded content must still match both hashes.
    let progress = progress
        .component(crate::DownloadComponent::Modpack)
        .with_size_hints();
    let (instances, folder) = instance_key(instance_dir)?;
    if database.modpack_header(instances, folder).await?.is_none()
        && !import_legacy_index(database, instances, folder, instance_dir).await?
    {
        return Ok(());
    }
    let files = database.modpack_files(instances, folder).await?;
    if files.is_empty() {
        return Ok(());
    }
    let mut verification = VerificationProgress::start(&progress, files.len()).await?;
    let mut pending = Vec::new();
    let mut downloaded_files = Vec::new();
    for file in &files {
        let task = file.task(instance_dir)?;
        let needs_download = prepare_download(
            &task.destination,
            &task.sha1,
            &file.sha512,
            &progress.control,
        )
        .await?;
        verification.checked().await?;
        if !needs_download {
            continue;
        }
        if let Some(parent) = task.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        pending.push(task);
        downloaded_files.push(file);
    }
    if pending.is_empty() {
        return Ok(());
    }
    progress
        .queue(
            client,
            &pending,
            crate::settings::download_options(game, settings).await?,
        )
        .await?;
    let mut verification = VerificationProgress::start(&progress, downloaded_files.len()).await?;
    for file in downloaded_files {
        ensure_sha512(
            &file.task(instance_dir)?.destination,
            &file.sha512,
            &progress.control,
        )
        .await?;
        verification.checked().await?;
    }
    Ok(())
}

/// Moves a v0.2.x `licht-modpack.json` into SQLite and reports whether it did.
/// An invalid file is kept.
///
/// The rows only stay if the file is removed, so rows and file never coexist.
async fn import_legacy_index(
    database: &crate::DatabaseClient,
    instances: &Path,
    folder: &str,
    instance_dir: &Path,
) -> Result<bool> {
    let path = instance_dir.join(MODPACK_INDEX_FILE);
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    // A CurseForge index, stored or still waiting in its legacy file, decides the instance.
    if instance_dir.join(CURSEFORGE_INDEX_FILE).exists()
        || !database
            .curseforge_files(instances, folder)
            .await?
            .is_empty()
    {
        return Ok(false);
    }
    let index = legacy_index(&body, instance_dir)
        .map_err(|error| CoreError::ModrinthLegacyIndex(Box::new(error)))?;
    database.replace_modpack(instances, folder, index).await?;
    if let Err(error) = remove_legacy_index(&path) {
        // The job fails either way; a failed undo leaves the rows, which still win over the file.
        // Deleting also clears CurseForge rows, which this instance was checked not to have.
        if let Err(undo) = database.delete_modpack(instances, folder).await {
            tracing::warn!(error = %undo, "could not undo a Modrinth index import");
        }
        return Err(error.into());
    }
    Ok(true)
}

fn legacy_index(body: &str, instance_dir: &Path) -> Result<StoredModpack> {
    let index: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    StoredModpack::parse(&index, instance_dir)
}

pub(crate) fn mrpack_download_client() -> Result<Client> {
    allowlisted_download_client(
        MRPACK_DOWNLOAD_HOSTS,
        "modpack redirect target is not allowed",
        "too many modpack redirects",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::storage::StoredCurseforgeFile;
    use crate::modpacks::mrpack::storage::StoredModpackHeader;
    use sha2::{Digest, Sha512};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn final_modrinth_verification_only_checks_new_downloads_and_enforces_sha512() {
        for valid_sha512 in [true, false] {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("licht-pack-verification-{stamp}"));
            let home = SharedCache::at(&root);
            let game = SharedCache::at(root.join("data"));
            let directory = game.instance_dir("Pack").expect("instance");
            std::fs::create_dir_all(directory.join("mods")).expect("mods");
            let body = b"hello world";
            std::fs::write(directory.join("mods/cached.jar"), body).expect("cached mod");
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let url = format!("http://{}/file", listener.local_addr().expect("address"));
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.expect("request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).await.expect("headers");
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world").await.expect("body");
            });
            let sha512 = crate::hashes::hex_encode(Sha512::digest(body).as_slice());
            let index = serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[
                {"path":"mods/cached.jar","fileSize":11,"hashes":{"sha1":crate::hashes::sha1_hex(body),"sha512":sha512.clone()},"downloads":["https://cdn.modrinth.com/data/fixture/cached.jar"]},
                {"path":"mods/new.jar","fileSize":12,"hashes":{"sha1":crate::hashes::sha1_hex(body),"sha512":if valid_sha512 { sha512 } else { "a".repeat(128) }},"downloads":["https://cdn.modrinth.com/data/fixture/new.jar"]}
            ]});
            let mut stored =
                crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory)
                    .expect("index");
            for file in &mut stored.files {
                file.url = url.clone();
            }
            let database = crate::Database::open(&home).await.expect("database");
            let service = crate::DatabaseService::start(database.clone())
                .await
                .expect("database");
            service
                .client()
                .replace_modpack(&game.instances_dir().expect("instances"), "Pack", stored)
                .await
                .expect("index");
            let (sender, mut incoming) = tokio::sync::mpsc::channel(128);
            let result = install_files(
                &service.client(),
                &game,
                &database,
                &directory,
                sender.into(),
                &Client::new(),
            )
            .await;
            server.await.expect("server");
            if valid_sha512 {
                result.expect("verified download despite stale size");
            } else {
                assert!(matches!(result, Err(CoreError::Sha512Mismatch { .. })));
            }
            let mut final_verification = None;
            while let Ok(progress) = incoming.try_recv() {
                if progress.bytes_total == 0 {
                    final_verification = Some(progress);
                }
            }
            let progress = final_verification.expect("verification progress");
            assert_eq!(
                progress.total, 1,
                "the cached file was already checked before downloading"
            );
            assert_eq!(progress.finished, u32::from(valid_sha512));
            assert_eq!(directory.join("mods/new.jar").exists(), valid_sha512);
            assert_eq!(
                std::fs::read(directory.join("mods/cached.jar")).expect("cached mod"),
                body
            );
            service.shutdown().await.expect("shutdown");
            std::fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[tokio::test]
    async fn imports_a_legacy_index_into_sqlite_and_removes_the_file() {
        let root = temp_dir("legacy");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        std::fs::write(instance.join(MODPACK_INDEX_FILE), LEGACY_INDEX).unwrap();
        let service = start_database(&cache).await;
        let client = service.client();
        // The second run reads SQLite only, so it must not need the removed file.
        for _ in 0..2 {
            install_cached(&client, &cache, &instance)
                .await
                .expect("cached files should not be downloaded");
            assert!(!instance.join(MODPACK_INDEX_FILE).exists());
        }
        let header = client
            .modpack_header(&root, "instance")
            .await
            .expect("header");
        let mut dependencies = client
            .modpack_dependencies(&root, "instance")
            .await
            .expect("dependencies");
        let files = client
            .modpack_files(&root, "instance")
            .await
            .expect("files");
        service.shutdown().await.expect("shutdown");
        assert_eq!(
            header,
            Some(StoredModpackHeader {
                format_version: Some(1),
                game: Some("minecraft".into()),
                version_id: Some("1.0.0".into()),
                name: Some("Fixture Pack".into()),
                summary: Some("A synthetic pack".into()),
            })
        );
        dependencies.sort();
        assert_eq!(
            dependencies,
            [
                ("fabric-loader".to_string(), "0.16.10".to_string()),
                ("minecraft".to_string(), "1.20.1".to_string()),
            ]
        );
        assert_eq!(
            files
                .iter()
                .map(|file| (
                    file.relative_path.as_str(),
                    file.sha1.as_str(),
                    file.sha512.as_str(),
                    file.url.as_str(),
                    file.size
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "mods/sodium.jar",
                    SHA1,
                    SHA512,
                    "https://cdn.modrinth.com/data/AANobbMI/versions/fixture/sodium.jar",
                    11
                ),
                (
                    "shaderpacks/shaders.zip",
                    SHA1,
                    SHA512,
                    "https://cdn.modrinth.com/data/fixture/versions/fixture/shaders.zip",
                    11
                ),
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn keeps_an_invalid_legacy_index_and_stores_nothing() {
        let root = temp_dir("legacy-invalid");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        let service = start_database(&cache).await;
        let client = service.client();
        let bodies = [
            "not json".to_string(),
            LEGACY_INDEX.replace("\"mods/sodium.jar\"", "\"../sodium.jar\""),
            LEGACY_INDEX.replace("https://cdn.modrinth.com", "https://example.com"),
            LEGACY_INDEX.replace("\"sha512\"", "\"sha256\""),
        ];
        for body in &bodies {
            std::fs::write(instance.join(MODPACK_INDEX_FILE), body).unwrap();
            let installed = install_cached(&client, &cache, &instance).await;
            let header = client
                .modpack_header(&root, "instance")
                .await
                .expect("header");
            assert!(
                matches!(installed, Err(CoreError::ModrinthLegacyIndex(_))),
                "{installed:?}"
            );
            assert!(header.is_none());
            assert_eq!(
                &std::fs::read_to_string(instance.join(MODPACK_INDEX_FILE)).unwrap(),
                body
            );
        }
        service.shutdown().await.expect("shutdown");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn stored_rows_take_precedence_over_a_leftover_legacy_index() {
        let root = temp_dir("legacy-leftover");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        std::fs::write(instance.join(MODPACK_INDEX_FILE), "not json").unwrap();
        let service = start_database(&cache).await;
        let client = service.client();
        client
            .replace_modpack(&root, "instance", fixture_index(&instance))
            .await
            .expect("stored index");
        let installed = install_cached(&client, &cache, &instance).await;
        let files = client
            .modpack_files(&root, "instance")
            .await
            .expect("files");
        service.shutdown().await.expect("shutdown");
        installed.expect("the leftover file must not be parsed");
        assert_eq!(files, fixture_index(&instance).files);
        assert_eq!(
            std::fs::read_to_string(instance.join(MODPACK_INDEX_FILE)).unwrap(),
            "not json"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ignores_a_legacy_index_in_a_curseforge_instance() {
        let root = temp_dir("legacy-curseforge");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        std::fs::write(instance.join(MODPACK_INDEX_FILE), LEGACY_INDEX).unwrap();
        let service = start_database(&cache).await;
        let client = service.client();
        let curseforge = vec![StoredCurseforgeFile {
            relative_path: "mods/sodium.jar".into(),
            url: "https://edge.forgecdn.net/files/1/2/sodium.jar".into(),
            sha1: Some(SHA1.into()),
            md5: None,
            size: 11,
            project_id: 1,
            file_id: 2,
            file_name: "sodium.jar".into(),
            name: "Sodium".into(),
            required: true,
        }];
        client
            .replace_curseforge_files(&root, "instance", curseforge.clone())
            .await
            .expect("CurseForge files");
        let installed = install_cached(&client, &cache, &instance).await;
        let header = client
            .modpack_header(&root, "instance")
            .await
            .expect("header");
        let stored = client
            .curseforge_files(&root, "instance")
            .await
            .expect("CurseForge files");
        service.shutdown().await.expect("shutdown");
        installed.expect("a CurseForge instance has no Modrinth files");
        assert!(header.is_none());
        assert_eq!(stored, curseforge);
        assert!(instance.join(MODPACK_INDEX_FILE).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn leaves_a_legacy_index_alone_while_a_curseforge_one_waits_for_import() {
        let root = temp_dir("legacy-both");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        std::fs::write(instance.join(MODPACK_INDEX_FILE), LEGACY_INDEX).unwrap();
        std::fs::write(
            instance.join(crate::modpacks::CURSEFORGE_INDEX_FILE),
            "not json",
        )
        .unwrap();
        let service = start_database(&cache).await;
        let client = service.client();
        let installed = install_cached(&client, &cache, &instance).await;
        let header = client
            .modpack_header(&root, "instance")
            .await
            .expect("header");
        service.shutdown().await.expect("shutdown");
        installed.expect("the CurseForge import decides this instance");
        assert!(header.is_none());
        assert!(instance.join(MODPACK_INDEX_FILE).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn keeps_the_legacy_index_and_stores_nothing_when_it_cannot_be_removed() {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 1;
        let root = temp_dir("legacy-locked");
        let cache = SharedCache::at(root.join("cache"));
        let instance = root.join("instance");
        place_pack_files(&instance);
        let index = instance.join(MODPACK_INDEX_FILE);
        std::fs::write(&index, LEGACY_INDEX).unwrap();
        let service = start_database(&cache).await;
        let client = service.client();
        // Readers may open the index, but nobody may delete it while this handle is open.
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&index)
            .expect("locked index");
        let blocked = install_cached(&client, &cache, &instance).await;
        let header_while_locked = client
            .modpack_header(&root, "instance")
            .await
            .expect("header");
        let files_while_locked = client
            .modpack_files(&root, "instance")
            .await
            .expect("files");
        drop(locked);
        let retried = install_cached(&client, &cache, &instance).await;
        let files_after_retry = client
            .modpack_files(&root, "instance")
            .await
            .expect("files");
        service.shutdown().await.expect("shutdown");
        assert!(matches!(blocked, Err(CoreError::Io(_))), "{blocked:?}");
        assert!(header_while_locked.is_none());
        assert!(files_while_locked.is_empty());
        retried.expect("the retry imports the index");
        assert_eq!(files_after_retry.len(), 2);
        assert!(!index.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    const LEGACY_INDEX: &str = include_str!("../../../tests/fixtures/modrinth/licht-modpack.json");
    const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const SHA512: &str = "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f";

    fn fixture_index(instance: &Path) -> StoredModpack {
        StoredModpack::parse(
            &serde_json::from_str(LEGACY_INDEX).expect("fixture"),
            instance,
        )
        .expect("fixture index")
    }

    /// Writes the fixture's client files so the install finds them cached.
    fn place_pack_files(instance: &Path) {
        for path in ["mods/sodium.jar", "shaderpacks/shaders.zip"] {
            let file = instance.join(path);
            std::fs::create_dir_all(file.parent().expect("parent")).unwrap();
            std::fs::write(file, b"hello world").unwrap();
        }
    }

    async fn start_database(cache: &SharedCache) -> crate::DatabaseService {
        crate::DatabaseService::start(crate::Database::open(cache).await.expect("database"))
            .await
            .expect("database")
    }

    /// Runs the install with a client that cannot reach any host.
    async fn install_cached(
        database: &crate::DatabaseClient,
        cache: &SharedCache,
        instance: &Path,
    ) -> Result<()> {
        let (progress, _incoming) = tokio::sync::mpsc::channel(64);
        let settings = crate::Database::open(cache).await.expect("database");
        let installed = install_files(
            database,
            cache,
            &settings,
            instance,
            progress.into(),
            &Client::new(),
        )
        .await;
        settings.close().await;
        installed
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-mrpack-install-{label}-{}-{stamp}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}
