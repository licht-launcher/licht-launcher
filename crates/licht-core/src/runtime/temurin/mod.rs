//! Eclipse Temurin (Adoptium) Java install and discovery.

mod api;
mod download;
mod layout;

pub(crate) use layout::{find_temurin, temurin_folder_name};

use std::path::PathBuf;

use tokio::sync::mpsc;

use super::archive::{extract_java_archive, java_executable, mark_executable};
use super::require::required_java_major;
use crate::{CoreError, DownloadProgress, LaunchEnvironment, Result, SharedCache, Version};

pub const ADOPTIUM_API: &str = "https://api.adoptium.net";

pub fn installed_java(
    cache: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
) -> Result<PathBuf> {
    let Some(root) = find_temurin(cache, required_java_major(version_id, version), environment)?
    else {
        return Err(CoreError::JavaRuntimeMissing);
    };
    Ok(java_executable(&root, environment.os))
}

pub async fn install_temurin(
    client: &reqwest::Client,
    cache: &SharedCache,
    major: u32,
    environment: &LaunchEnvironment,
    api_base: &str,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf> {
    if let Some(root) = find_temurin(cache, major, environment)? {
        return Ok(java_executable(&root, environment.os));
    }

    let release =
        api::temurin_package(client, api_base, major, environment.os, environment.arch).await?;
    let root = layout::runtime_folder(
        cache,
        &temurin_folder_name(
            major,
            &release.image,
            &release.version,
            environment.os,
            environment.arch,
        )?,
    )?;
    let executable = java_executable(&root, environment.os);
    std::fs::create_dir_all(&root)?;

    let package = &release.package;
    let name = download::archive_name(&package.name)?;
    let archive = root.join(name);
    download::download_sha256(
        client,
        &package.link,
        &archive,
        &package.checksum,
        package.size,
        &progress,
    )
    .await?;
    let extracted = extract_java_archive(&archive, &root);
    let _ = std::fs::remove_file(&archive);
    extracted?;
    if !executable.is_file() {
        return Err(CoreError::JavaRuntimeMissing);
    }
    mark_executable(&executable)?;
    let _ = progress.try_send(DownloadProgress {
        finished: 1,
        failed: 0,
        total: 1,
        bytes_done: package.size,
        bytes_total: package.size,
    });
    Ok(executable)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::install_temurin;
    use super::layout::platform_label;
    use crate::hashes::hex_encode;
    use crate::{Arch, CoreError, DownloadProgress, LaunchEnvironment, OsName, SharedCache};

    #[tokio::test]
    async fn temurin_is_stored_under_its_major_version() {
        let _guard = crate::test_http::lock_local_http().await;
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let cache = SharedCache::at(&directory);
        let zip = sample_zip();
        let checksum = hex_encode(Sha256::digest(&zip).as_slice());
        let server = serve(zip, &checksum, false, "21.0.4+7").await;
        let (progress, mut incoming) = tokio::sync::mpsc::channel::<DownloadProgress>(8);
        let executable = install_temurin(
            &reqwest::Client::new(),
            &cache,
            21,
            &environment(),
            &server,
            progress,
        )
        .await
        .expect("temurin");
        while incoming.recv().await.is_some() {}

        let folder = format!(
            "temurin21-jre21.0.4-{}",
            platform_label(environment().os, environment().arch)
        );
        let expected = cache
            .root()
            .join("runtime")
            .join(&folder)
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        assert_eq!(executable, expected);
        assert_eq!(std::fs::read(&expected).expect("java"), b"temurin");
        assert!(
            !expected
                .parent()
                .expect("bin")
                .parent()
                .expect("root")
                .join("jdk-21")
                .exists()
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn a_missing_jre_falls_back_to_the_jdk() {
        let _guard = crate::test_http::lock_local_http().await;
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-jdk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let cache = SharedCache::at(&directory);
        let zip = sample_zip();
        let checksum = hex_encode(Sha256::digest(&zip).as_slice());
        let server = serve(zip, &checksum, true, "25.0.2+10").await;
        let executable = install_temurin(
            &reqwest::Client::new(),
            &cache,
            25,
            &environment(),
            &server,
            tokio::sync::mpsc::channel(8).0,
        )
        .await
        .expect("jdk fallback");
        let folder = format!(
            "temurin25-jdk25.0.2-{}",
            platform_label(environment().os, environment().arch)
        );
        let expected = cache
            .root()
            .join("runtime")
            .join(folder)
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        assert_eq!(executable, expected);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn a_wrong_checksum_is_rejected() {
        let _guard = crate::test_http::lock_local_http().await;
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let cache = SharedCache::at(&directory);
        let server = serve(sample_zip(), &"ab".repeat(32), false, "21.0.4+7").await;
        let error = install_temurin(
            &reqwest::Client::new(),
            &cache,
            21,
            &environment(),
            &server,
            tokio::sync::mpsc::channel(8).0,
        )
        .await
        .expect_err("checksum");
        assert!(matches!(error, CoreError::Sha256Mismatch { .. }));
        let _ = std::fs::remove_dir_all(directory);
    }

    fn environment() -> LaunchEnvironment {
        LaunchEnvironment {
            os: if cfg!(windows) {
                OsName::Windows
            } else {
                OsName::Linux
            },
            arch: Arch::X86_64,
            os_version: String::new(),
            features: std::collections::BTreeMap::new(),
        }
    }

    fn sample_zip() -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let name = if cfg!(windows) {
            "jdk-21/bin/java.exe"
        } else {
            "jdk-21/bin/java"
        };
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .expect("zip entry");
        zip.write_all(b"temurin").expect("zip bytes");
        zip.finish().expect("zip");
        cursor.into_inner()
    }

    async fn serve(zip: Vec<u8>, checksum: &str, jre_missing: bool, semver: &str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        let base = format!("http://{address}");
        let checksum = std::sync::Arc::new(checksum.to_string());
        let semver = std::sync::Arc::new(semver.to_string());
        let size = zip.len();
        let zip = std::sync::Arc::new(zip);
        let shared_base = std::sync::Arc::new(base.clone());
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let zip = std::sync::Arc::clone(&zip);
                let checksum = std::sync::Arc::clone(&checksum);
                let semver = std::sync::Arc::clone(&semver);
                let base = std::sync::Arc::clone(&shared_base);
                tokio::spawn(async move {
                    let mut incoming = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        incoming.extend_from_slice(&buffer[..read]);
                        if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request = String::from_utf8_lossy(&incoming);
                    let path = request
                        .lines()
                        .next()
                        .unwrap_or("")
                        .split(' ')
                        .nth(1)
                        .unwrap_or("");
                    let image = if path.contains("image_type=jdk") {
                        "jdk"
                    } else {
                        "jre"
                    };
                    let package = format!(
                        r#"[{{"binary":{{"image_type":"{image}","package":{{"checksum":"{checksum}","link":"{base}/temurin.zip","name":"temurin.zip","size":{size}}}}},"version":{{"semver":"{semver}"}}}}]"#
                    );
                    let body = if path.contains("image_type=jre") && jre_missing {
                        b"[]".to_vec()
                    } else if path.contains("image_type=") {
                        package.into_bytes()
                    } else {
                        zip.as_ref().clone()
                    };
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    if socket.write_all(header.as_bytes()).await.is_err() {
                        return;
                    }
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        base
    }
}
