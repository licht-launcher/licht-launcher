use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use super::archive::{extract_java_archive, java_executable, mark_executable};
use super::require::required_java_major;
use crate::hashes::hex_encode;
use crate::{
    Arch, CoreError, DownloadProgress, LaunchEnvironment, OsName, Result, SharedCache, Version,
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
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf> {
    if let Some(root) = find_temurin(cache, major, environment)? {
        return Ok(java_executable(&root, environment.os));
    }

    let release =
        temurin_package(client, api_base, major, environment.os, environment.arch).await?;
    let root = runtime_folder(
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
    let name = archive_name(&package.name)?;
    let archive = root.join(name);
    download_sha256(
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

fn runtime_folder(cache: &SharedCache, name: &str) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(CoreError::CachePath);
    }
    Ok(cache.root().join("runtime").join(name))
}

/// `temurin25-jre25.0.2-win_x64`: one folder, with the package and the machine in the name.
pub(crate) fn temurin_folder_name(
    major: u32,
    image: &str,
    version: &str,
    os: OsName,
    arch: Arch,
) -> Result<String> {
    if image != "jre" && image != "jdk" {
        return Err(CoreError::JavaRuntimeMissing);
    }
    if version.is_empty()
        || version.contains(['/', '\\', '-'])
        || version
            .split('.')
            .any(|part| part.is_empty() || part.parse::<u32>().is_err())
    {
        return Err(CoreError::JavaRuntimeMissing);
    }
    Ok(format!(
        "temurin{major}-{image}{version}-{}",
        platform_label(os, arch)
    ))
}

pub(crate) fn platform_label(os: OsName, arch: Arch) -> String {
    let os = match os {
        OsName::Windows => "win",
        OsName::Linux => "linux",
        OsName::Osx => "mac",
    };
    let arch = match arch {
        Arch::X86_64 => "x64",
        Arch::X86 => "x86",
    };
    format!("{os}_{arch}")
}

pub(crate) fn find_temurin(
    cache: &SharedCache,
    major: u32,
    environment: &LaunchEnvironment,
) -> Result<Option<PathBuf>> {
    let runtime = cache.root().join("runtime");
    let platform = platform_label(environment.os, environment.arch);
    let entries = match std::fs::read_dir(&runtime) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if folder_middle(&name, major, &platform).is_none() {
            continue;
        }
        if java_executable(&path, environment.os).is_file() {
            found.push((name, path));
        }
    }
    found.sort_by_key(|left| std::cmp::Reverse(temurin_rank(&left.0)));
    Ok(found.into_iter().next().map(|(_, path)| path))
}

fn folder_middle<'a>(name: &'a str, major: u32, platform: &str) -> Option<&'a str> {
    let rest = name.strip_prefix(&format!("temurin{major}-"))?;
    let middle = rest.strip_suffix(&format!("-{platform}"))?;
    if middle.starts_with("jre") || middle.starts_with("jdk") {
        Some(middle)
    } else {
        None
    }
}

fn temurin_rank(name: &str) -> (bool, Vec<u32>) {
    let middle = name.split('-').nth(1).unwrap_or("");
    let (jre, version) = if let Some(version) = middle.strip_prefix("jre") {
        (true, version)
    } else if let Some(version) = middle.strip_prefix("jdk") {
        (false, version)
    } else {
        (false, "")
    };
    let parts = version
        .split('.')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    (jre, parts)
}

fn adoptium_os(os: OsName) -> &'static str {
    match os {
        OsName::Windows => "windows",
        OsName::Linux => "linux",
        OsName::Osx => "mac",
    }
}

fn adoptium_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x64",
        Arch::X86 => "x86",
    }
}

fn assets_url(api_base: &str, major: u32, os: &str, arch: &str, image: &str) -> String {
    format!(
        "{}/v3/assets/latest/{major}/hotspot?architecture={arch}&image_type={image}&os={os}&vendor=eclipse",
        api_base.trim_end_matches('/')
    )
}

async fn temurin_package(
    client: &reqwest::Client,
    api_base: &str,
    major: u32,
    os: OsName,
    arch: Arch,
) -> Result<TemurinRelease> {
    let os_name = adoptium_os(os);
    let arch_name = adoptium_arch(arch);
    if let Some(release) = fetch_assets(client, api_base, major, os_name, arch_name, "jre").await? {
        return Ok(release);
    }
    fetch_assets(client, api_base, major, os_name, arch_name, "jdk")
        .await?
        .ok_or(CoreError::JavaRuntimeMissing)
}

async fn fetch_assets(
    client: &reqwest::Client,
    api_base: &str,
    major: u32,
    os: &str,
    arch: &str,
    image: &str,
) -> Result<Option<TemurinRelease>> {
    let body = client
        .get(assets_url(api_base, major, os, arch, image))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let assets: Vec<TemurinAsset> = serde_json::from_str(&body).map_err(CoreError::JavaRuntime)?;
    let Some(asset) = assets.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(TemurinRelease {
        image: asset.binary.image_type,
        version: release_number(&asset.version.semver)?,
        package: asset.binary.package,
    }))
}

fn release_number(semver: &str) -> Result<String> {
    let version = semver
        .split_once('+')
        .map_or(semver, |(version, _)| version);
    if version.is_empty() {
        return Err(CoreError::JavaRuntimeMissing);
    }
    Ok(version.to_string())
}

#[derive(Debug, Deserialize)]
struct TemurinAsset {
    binary: TemurinBinary,
    version: TemurinVersion,
}

#[derive(Debug, Deserialize)]
struct TemurinBinary {
    image_type: String,
    package: TemurinPackage,
}

#[derive(Debug, Deserialize)]
struct TemurinVersion {
    semver: String,
}

struct TemurinRelease {
    image: String,
    version: String,
    package: TemurinPackage,
}

#[derive(Debug, Clone, Deserialize)]
struct TemurinPackage {
    checksum: String,
    link: String,
    name: String,
    size: u64,
}

fn archive_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(CoreError::CachePath);
    }
    Ok(name)
}

async fn download_sha256(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_sha256: &str,
    size: u64,
    progress: &mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let response = client.get(url).send().await?.error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(destination).await?;
    let mut hasher = Sha256::new();
    let mut done = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        done = done.saturating_add(chunk.len() as u64);
        let _ = progress.try_send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: 1,
            bytes_done: done,
            bytes_total: size,
        });
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);
    let actual = hex_encode(hasher.finalize().as_slice());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        let _ = std::fs::remove_file(destination);
        return Err(CoreError::Sha256Mismatch {
            expected: expected_sha256.to_string(),
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use sha2::{Digest, Sha256};

    use super::{install_temurin, platform_label, temurin_folder_name};
    use crate::hashes::hex_encode;
    use crate::{Arch, CoreError, DownloadProgress, LaunchEnvironment, OsName, SharedCache};

    #[test]
    fn a_temurin_folder_names_the_package_and_the_machine() {
        assert_eq!(
            temurin_folder_name(25, "jre", "25.0.2", OsName::Windows, Arch::X86_64).expect("name"),
            "temurin25-jre25.0.2-win_x64"
        );
        assert_eq!(
            temurin_folder_name(8, "jdk", "8.0.422", OsName::Linux, Arch::X86).expect("name"),
            "temurin8-jdk8.0.422-linux_x86"
        );
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
