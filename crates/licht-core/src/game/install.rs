use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};
use tokio::sync::mpsc;

use crate::download::DownloadProgress;
use crate::natives::{classifier_matches, native_classifier};
use crate::{
    ADOPTIUM_API, AssetIndexFile, CoreError, DEFAULT_RETRY, DownloadTask, LaunchEnvironment,
    Result, SharedCache, Version, applicable_libraries, download_all, download_concurrency,
    install_temurin, parse_asset_index, parse_version, required_java_major, use_custom_java,
};

pub const ASSET_OBJECT_BASE: &str = "https://resources.download.minecraft.net";

pub struct InstallPlan<'a> {
    pub version_id: &'a str,
    pub version: &'a Version,
    pub asset_index_json: &'a str,
    pub assets: &'a AssetIndexFile,
    pub environment: &'a LaunchEnvironment,
    pub asset_base: &'a str,
}

pub async fn install_version(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    plan: InstallPlan<'_>,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    cache.create()?;
    write_asset_index(cache, plan.version, plan.asset_index_json)?;

    let mut pending = Vec::new();
    for task in download_tasks(cache, &plan)? {
        if file_matches(&task.destination, &task.sha1)? {
            continue;
        }
        if let Some(parent) = task.destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        pending.push(task);
    }

    download_all(
        client,
        &pending,
        crate::shared_transfer_limits(),
        download_concurrency(cache, settings)?,
        crate::write_concurrency(cache, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await
}

pub struct GameInstall<'a> {
    pub version_id: &'a str,
    pub version_json_url: &'a str,
    pub version_sha1: &'a str,
    pub environment: &'a LaunchEnvironment,
    pub asset_base: &'a str,
    pub java: Option<&'a Path>,
}

#[derive(Debug)]
pub struct InstallArgs {
    pub version_id: String,
    pub cache: Option<PathBuf>,
    pub java: Option<PathBuf>,
}

pub fn parse_install_args(args: &[String]) -> Result<InstallArgs> {
    if args.first().map(String::as_str) != Some("install") {
        return Err(CoreError::LaunchArgs);
    }

    let mut version_id = None;
    let mut cache = None;
    let mut java = None;
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            return Err(CoreError::LaunchArgs);
        };
        if value.starts_with("--") {
            return Err(CoreError::LaunchArgs);
        }
        match flag.as_str() {
            "--version" => version_id = Some(value.clone()),
            "--cache" => cache = Some(PathBuf::from(value)),
            "--java" => java = Some(PathBuf::from(value)),
            _ => return Err(CoreError::LaunchArgs),
        }
    }

    let Some(version_id) = version_id.filter(|id| !id.is_empty()) else {
        return Err(CoreError::LaunchArgs);
    };
    Ok(InstallArgs {
        version_id,
        cache,
        java,
    })
}

/// Downloads one version from the URLs in `request` and installs Eclipse Temurin.
/// A custom `java` skips that download.
pub async fn install_game(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    request: GameInstall<'_>,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf> {
    let json = read_text(client, request.version_json_url).await?;
    let actual = sha1_hex(json.as_bytes());
    if !actual.eq_ignore_ascii_case(request.version_sha1) {
        return Err(CoreError::Sha1Mismatch {
            expected: request.version_sha1.to_string(),
            actual,
        });
    }
    let version = parse_version(&json)?;
    let json_path = cache.version_json(request.version_id)?;
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&json_path, &json)?;

    let index_json = read_text(client, &version.asset_index.url).await?;
    let assets = parse_asset_index(&index_json)?;
    install_version(
        client,
        cache,
        settings,
        InstallPlan {
            version_id: request.version_id,
            version: &version,
            asset_index_json: &index_json,
            assets: &assets,
            environment: request.environment,
            asset_base: request.asset_base,
        },
        progress.clone(),
    )
    .await?;

    if let Some(java) = request.java {
        return use_custom_java(java);
    }

    let major = required_java_major(request.version_id, &version);
    install_temurin(
        client,
        cache,
        major,
        request.environment,
        ADOPTIUM_API,
        progress,
    )
    .await
}

async fn read_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let mut last_error = None;
    for _ in 0..3 {
        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };
        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };
        match response.text().await {
            Ok(text) => return Ok(text),
            Err(error) => last_error = Some(error),
        }
    }
    match last_error {
        Some(error) => Err(error.into()),
        None => Err(CoreError::JavaRuntimeMissing),
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    hex_encode(Sha1::digest(bytes).as_slice())
}

fn write_asset_index(cache: &SharedCache, version: &Version, json: &str) -> Result<()> {
    let expected = &version.asset_index.sha1;
    let actual = hex_encode(Sha1::digest(json.as_bytes()).as_slice());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(CoreError::Sha1Mismatch {
            expected: expected.clone(),
            actual,
        });
    }

    let path = cache.asset_index(&version.asset_index.id)?;
    if file_matches(&path, expected)? {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, json)?;
    Ok(())
}

fn download_tasks(cache: &SharedCache, plan: &InstallPlan<'_>) -> Result<Vec<DownloadTask>> {
    let mut tasks = Vec::new();
    tasks.push(DownloadTask {
        url: plan.version.downloads.client.url.clone(),
        destination: cache.version_jar(plan.version_id)?,
        sha1: plan.version.downloads.client.sha1.clone(),
        size: plan.version.downloads.client.size,
    });

    for library in applicable_libraries(plan.version, plan.environment) {
        if other_platform_native(&library.name, plan.environment) {
            continue;
        }
        let Some(downloads) = &library.downloads else {
            continue;
        };
        if let Some(artifact) = &downloads.artifact {
            tasks.push(task_for_artifact(cache, artifact)?);
        }
        let Some(natives) = &library.natives else {
            continue;
        };
        let Some(template) = natives.get(plan.environment.os.mojang_name()) else {
            continue;
        };
        let Some(classifiers) = &downloads.classifiers else {
            continue;
        };
        let classifier_name = native_classifier(template, plan.environment.arch);
        let Some(artifact) = classifiers.get(&classifier_name) else {
            continue;
        };
        tasks.push(task_for_artifact(cache, artifact)?);
    }

    let base = plan.asset_base.trim_end_matches('/');
    for object in plan.assets.objects.values() {
        let hash = object.hash.to_ascii_lowercase();
        let destination = cache.asset_object(&hash)?;
        let prefix = &hash[..2];
        tasks.push(DownloadTask {
            url: format!("{base}/{prefix}/{hash}"),
            destination,
            sha1: hash,
            size: object.size,
        });
    }

    Ok(tasks)
}

/// `natives-windows-x86` and `natives-linux` are other machines. The plain
/// `natives-windows` name is the 64-bit jar for Windows.
fn other_platform_native(name: &str, environment: &LaunchEnvironment) -> bool {
    let Some(classifier) = name.split(':').nth(3) else {
        return false;
    };
    classifier.starts_with("natives-")
        && !classifier_matches(classifier, environment.os, environment.arch)
}

fn task_for_artifact(cache: &SharedCache, artifact: &crate::Artifact) -> Result<DownloadTask> {
    Ok(DownloadTask {
        url: artifact.url.clone(),
        destination: cache.library(&artifact.path)?,
        sha1: artifact.sha1.clone(),
        size: artifact.size,
    })
}

fn file_matches(path: &Path, expected_sha1: &str) -> Result<bool> {
    // Same rule as the downloader: a missing hash must not skip the file.
    if expected_sha1.is_empty() {
        return Ok(false);
    }
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(hasher.finalize().as_slice()).eq_ignore_ascii_case(expected_sha1))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::file_matches;

    #[test]
    fn an_empty_sha1_does_not_treat_an_existing_file_as_verified() {
        let directory =
            std::env::temp_dir().join(format!("licht-install-empty-sha-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("temp directory");
        let path = directory.join("library.jar");
        std::fs::write(&path, b"unverified").expect("file");
        assert!(!file_matches(&path, "").expect("match"));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
