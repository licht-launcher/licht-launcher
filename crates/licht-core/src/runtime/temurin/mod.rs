//! Eclipse Temurin (Adoptium) Java install and discovery.

mod api;
mod download;
mod layout;

pub(crate) use layout::{find_temurin, temurin_folder_name};

use std::path::PathBuf;

use super::archive::{extract_java_archive, java_executable, mark_executable};
use super::require::required_java_major;
use crate::{
    CoreError, DownloadComponent, DownloadPhase, DownloadProgress, InstallationContext,
    LaunchEnvironment, Result, SharedCache, Version,
};

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
    progress: impl Into<InstallationContext>,
) -> Result<PathBuf> {
    let progress = progress.into().component(DownloadComponent::Java);
    progress.checkpoint().await?;
    progress.stage(DownloadPhase::Preparing).await?;
    if let Some(root) = find_temurin(cache, major, environment)? {
        return Ok(java_executable(&root, environment.os));
    }

    progress.stage(DownloadPhase::Preparing).await?;
    let release = progress
        .metadata(api::temurin_package(
            client,
            api_base,
            major,
            environment.os,
            environment.arch,
        ))
        .await?;
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
    let lock = crate::transfer::download::destination_lock(&root);
    let _guard = progress.metadata(async { Ok(lock.lock().await) }).await?;
    if let Some(installed) = find_temurin(cache, major, environment)? {
        return Ok(java_executable(&installed, environment.os));
    }
    let executable = java_executable(&root, environment.os);
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join(".installing"), b"")?;

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
    progress.stage(DownloadPhase::Extracting).await?;
    let extracted = extract_java_archive(&archive, &root);
    extracted?;
    if !executable.is_file() {
        return Err(CoreError::JavaRuntimeMissing);
    }
    mark_executable(&executable)?;
    progress.checkpoint().await?;
    std::fs::remove_file(&archive)?;
    std::fs::remove_file(root.join(".installing"))?;
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
    use crate::{
        Arch, CoreError, DownloadControl, DownloadProgress, InstallationContext, LaunchEnvironment,
        OsName, SharedCache,
    };

    #[tokio::test]
    async fn a_paused_java_installation_does_not_block_other_runtime_directories() {
        let _guard = crate::test_http::lock_local_http().await;
        for already_installed in [false, true] {
            let directory = std::env::temp_dir().join(format!(
                "licht-temurin-independent-{}-{already_installed}",
                std::process::id()
            ));
            let cache = SharedCache::at(directory.join("data"));
            let settings = SharedCache::at(directory.join("settings"));
            let zip = sample_zip();
            let checksum = hex_encode(Sha256::digest(&zip).as_slice());
            let first_api = serve(zip.clone(), &checksum, false, "8.0.4+7").await;
            let second_api = serve(zip, &checksum, false, "21.0.4+7").await;
            let expected_root = super::layout::runtime_folder(
                &cache,
                &super::temurin_folder_name(
                    21,
                    "jre",
                    "21.0.4",
                    environment().os,
                    environment().arch,
                )
                .expect("runtime name"),
            )
            .expect("runtime path");
            let expected =
                crate::runtime::archive::java_executable(&expected_root, environment().os);
            if already_installed {
                std::fs::create_dir_all(expected.parent().expect("bin"))
                    .expect("runtime directory");
                std::fs::write(&expected, b"temurin").expect("installed Java");
            }
            let job =
                crate::transfer::jobs::create_java_job(&settings, &cache, 8).expect("Java job");
            let control = DownloadControl::new();
            let (sender, mut events) = tokio::sync::mpsc::channel(64);
            let context = InstallationContext::for_job(
                settings,
                cache.clone(),
                job.id,
                control.clone(),
                sender,
            );
            let first_cache = cache.clone();
            let running = tokio::spawn(async move {
                install_temurin(
                    &reqwest::Client::new(),
                    &first_cache,
                    8,
                    &environment(),
                    &first_api,
                    context,
                )
                .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    if matches!(
                        events.recv().await.expect("installation event"),
                        crate::transfer::installation::InstallationEvent::Stage(
                            _,
                            crate::DownloadPhase::Downloading
                        )
                    ) {
                        control.pause();
                        break;
                    }
                }
            })
            .await
            .expect("first runtime reached its download phase");
            let independent = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                install_temurin(
                    &reqwest::Client::new(),
                    &cache,
                    21,
                    &environment(),
                    &second_api,
                    InstallationContext::default(),
                ),
            )
            .await;
            control.cancel();
            assert!(matches!(
                running.await.expect("first worker"),
                Err(CoreError::DownloadCancelled)
            ));
            assert_eq!(
                independent
                    .expect("another runtime must progress while Java 8 is paused")
                    .expect("independent Java"),
                expected
            );
            assert_eq!(std::fs::read(expected).expect("Java bytes"), b"temurin");
            std::fs::remove_dir_all(directory).expect("cleanup");
        }
    }

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
    async fn concurrent_installations_of_the_same_runtime_download_only_once() {
        let _guard = crate::test_http::lock_local_http().await;
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-shared-{}", std::process::id()));
        let cache = SharedCache::at(&directory);
        let zip = sample_zip();
        let checksum = hex_encode(Sha256::digest(&zip).as_slice());
        let (server, downloads) = serve_counted(zip, &checksum, false, "21.0.4+7").await;
        let client = reqwest::Client::new();
        let env = environment();
        let (first, second) = tokio::join!(
            install_temurin(
                &client,
                &cache,
                21,
                &env,
                &server,
                InstallationContext::default()
            ),
            install_temurin(
                &client,
                &cache,
                21,
                &env,
                &server,
                InstallationContext::default()
            ),
        );
        let first = first.expect("first install");
        assert_eq!(second.expect("second install"), first);
        assert_eq!(downloads.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(first).expect("Java"), b"temurin");
        std::fs::remove_dir_all(directory).expect("cleanup");
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
        serve_counted(zip, checksum, jre_missing, semver).await.0
    }

    async fn serve_counted(
        zip: Vec<u8>,
        checksum: &str,
        jre_missing: bool,
        semver: &str,
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let downloads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = std::sync::Arc::clone(&downloads);
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
                let downloads = std::sync::Arc::clone(&counted);
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
                        downloads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        (base, downloads)
    }
}
