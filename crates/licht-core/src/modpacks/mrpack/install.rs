use std::fs;
use std::path::Path;

use super::index::MRPACK_DOWNLOAD_HOSTS;
use crate::download::DEFAULT_RETRY;
use crate::modpacks::file_hashes::prepare_download;
use crate::modpacks::secure_download::allowlisted_download_client;
use crate::settings::{download_concurrency, write_concurrency};
use crate::transfer::verification::VerificationProgress;
use crate::{CoreError, Result, SharedCache, shared_transfer_limits};
use reqwest::Client;

/// Downloads every client-required file from the stored Modrinth index into the instance.
///
/// No-op when no files are stored (vanilla / CurseForge without mrpack files).
/// A redirect is followed only when that hop is HTTPS and the host is
/// `cdn.modrinth.com`.
pub async fn install_mrpack_files(
    database: &crate::DatabaseClient,
    game: &SharedCache,
    settings: &SharedCache,
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
    settings: &SharedCache,
    instance_dir: &Path,
    progress: crate::InstallationContext,
    client: &Client,
) -> Result<()> {
    // Pack indexes can contain stale sizes; downloaded content must still match both hashes.
    let progress = progress
        .component(crate::DownloadComponent::Modpack)
        .with_size_hints();
    let instances = instance_dir.parent().ok_or(CoreError::CachePath)?;
    let folder = instance_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CoreError::InstanceName)?;
    if database.modpack_header(instances, folder).await?.is_none() {
        return Ok(());
    }
    let files = database.modpack_files(instances, folder).await?;
    if files.is_empty() {
        return Ok(());
    }
    let mut verification = VerificationProgress::start(&progress, files.len()).await?;
    let mut pending = Vec::new();
    for file in &files {
        let task = file.task(instance_dir)?;
        let checksum = crate::DownloadChecksum::Sha1AndSha512 {
            sha1: task.sha1.clone(),
            sha512: file.sha512.clone(),
        };
        checksum.validate()?;
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
        pending.push(crate::VerifiedDownloadTask {
            url: task.url,
            destination: task.destination,
            checksum,
            size: task.size,
        });
    }
    if pending.is_empty() {
        return Ok(());
    }
    progress
        .verified_queue(
            client,
            &pending,
            crate::DownloadOptions {
                limits: shared_transfer_limits(),
                download_concurrency: download_concurrency(game, settings)?,
                write_concurrency: write_concurrency(game, settings)?,
                retry: DEFAULT_RETRY,
            },
        )
        .await?;
    Ok(())
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
    use sha2::{Digest, Sha512};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn modrinth_verifies_both_hashes_before_reporting_download_completion() {
        for (valid_sha1, valid_sha512, resume) in [
            (true, true, false),
            (false, true, false),
            (true, false, false),
            (false, false, false),
            (true, true, true),
            (false, true, true),
            (true, false, true),
            (false, false, true),
        ] {
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
            if resume {
                std::fs::write(directory.join("mods/new.jar.part"), b"hello").expect("partial");
            }
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let url = format!("http://{}/file", listener.local_addr().expect("address"));
            let server = tokio::spawn(async move {
                let attempts = if valid_sha1 && valid_sha512 {
                    1
                } else {
                    crate::DEFAULT_RETRY.attempts
                };
                for attempt in 0..attempts {
                    let (mut socket, _) = listener.accept().await.expect("request");
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        let count = socket.read(&mut buffer).await.expect("headers");
                        assert!(count > 0);
                        request.extend_from_slice(&buffer[..count]);
                    }
                    let request = String::from_utf8(request)
                        .expect("request headers")
                        .to_ascii_lowercase();
                    if resume && attempt == 0 {
                        assert!(request.contains("range: bytes=5-\r\n"));
                        socket.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 5-10/11\r\nContent-Length: 6\r\nConnection: close\r\n\r\n world").await.expect("suffix");
                    } else {
                        assert!(!request.contains("range:"));
                        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world").await.expect("body");
                    }
                }
            });
            let sha512 = crate::hashes::hex_encode(Sha512::digest(body).as_slice());
            let index = serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[
                {"path":"mods/cached.jar","fileSize":11,"hashes":{"sha1":crate::hashes::sha1_hex(body),"sha512":sha512.clone()},"downloads":["https://cdn.modrinth.com/data/fixture/cached.jar"]},
                {"path":"mods/new.jar","fileSize":if resume { 4 } else { 12 },"hashes":{"sha1":if valid_sha1 { crate::hashes::sha1_hex(body) } else { "0".repeat(40) },"sha512":if valid_sha512 { sha512 } else { "a".repeat(128) }},"downloads":["https://cdn.modrinth.com/data/fixture/new.jar"]}
            ]});
            let mut stored =
                crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory)
                    .expect("index");
            for file in &mut stored.files {
                file.url = url.clone();
            }
            let service = crate::DatabaseService::start(home.clone())
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
                &home,
                &directory,
                sender.into(),
                &Client::new(),
            )
            .await;
            let valid = valid_sha1 && valid_sha512;
            if valid {
                result.expect("verified download despite stale size");
            } else if !valid_sha1 {
                assert!(matches!(result, Err(CoreError::Sha1Mismatch { .. })));
            } else {
                assert!(matches!(result, Err(CoreError::Sha512Mismatch { .. })));
            }
            server.await.expect("server");
            let mut final_progress = None;
            let mut verification_totals = Vec::new();
            let mut reported_download_completion = false;
            while let Ok(progress) = incoming.try_recv() {
                reported_download_completion |= progress.bytes_total > 0 && progress.finished > 0;
                if progress.bytes_total == 0 {
                    verification_totals.push(progress.total);
                }
                final_progress = Some(progress);
            }
            assert_eq!(
                reported_download_completion, valid,
                "both hashes must pass before a download is published"
            );
            assert!(!verification_totals.is_empty());
            assert!(
                verification_totals.iter().all(|total| *total == 2),
                "verification only scans existing files before downloading"
            );
            let progress = final_progress.expect("final progress");
            assert_eq!((progress.total, progress.bytes_total), (1, 11));
            assert_eq!(
                (progress.finished, progress.failed),
                (u32::from(valid), u32::from(!valid))
            );
            assert_eq!(directory.join("mods/new.jar").exists(), valid);
            assert!(!directory.join("mods/new.jar.part").exists());
            assert_eq!(
                std::fs::read(directory.join("mods/cached.jar")).expect("cached mod"),
                body
            );
            service.shutdown().await.expect("shutdown");
            std::fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[tokio::test]
    async fn malformed_checksum_metadata_does_not_remove_existing_mods() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("licht-pack-invalid-hash-{stamp}"));
        let home = SharedCache::at(&root);
        let game = SharedCache::at(root.join("data"));
        let directory = game.instance_dir("Pack").expect("instance");
        std::fs::create_dir_all(directory.join("mods")).expect("mods");
        let body = b"existing mod";
        let path = directory.join("mods/cached.jar");
        std::fs::write(&path, body).expect("mod");
        let index = serde_json::json!({"dependencies":{"minecraft":"1.20.1"},"files":[{
            "path":"mods/cached.jar","fileSize":body.len(),
            "hashes":{"sha1":crate::hashes::sha1_hex(body),"sha512":"invalid"},
            "downloads":["https://cdn.modrinth.com/data/fixture/cached.jar"]
        }]});
        let stored = crate::modpacks::mrpack::storage::StoredModpack::parse(&index, &directory)
            .expect("index");
        let service = crate::DatabaseService::start(home.clone())
            .await
            .expect("database");
        service
            .client()
            .replace_modpack(&game.instances_dir().expect("instances"), "Pack", stored)
            .await
            .expect("index");
        let result = install_files(
            &service.client(),
            &game,
            &home,
            &directory,
            crate::InstallationContext::default(),
            &Client::new(),
        )
        .await;
        assert!(matches!(
            result,
            Err(CoreError::DownloadResponse {
                detail: "invalid published checksum"
            })
        ));
        assert_eq!(std::fs::read(&path).expect("mod preserved"), body);
        service.shutdown().await.expect("shutdown");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
