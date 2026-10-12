use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use tokio::sync::mpsc;
use ts_rs::TS;

use crate::{
    Arch, CoreError, DEFAULT_RETRY, DownloadProgress, DownloadTask, LaunchEnvironment, OsName,
    Result, SharedCache, Version, download_all, download_concurrency,
};

pub const JAVA_RUNTIME_INDEX_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
pub const ADOPTIUM_API: &str = "https://api.adoptium.net";

/// The runtime a version asks for, when the JSON includes `javaVersion`.
pub fn required_runtime(version: &Version) -> Option<&crate::JavaVersion> {
    version.java_version.as_ref()
}

/// Major version Eclipse Temurin should provide.
///
/// `javaVersion.majorVersion` wins. Without it, the release id decides:
/// 1.17 is 16, 1.18 through 1.20.4 is 17, 1.20.5 and newer is 21, and every
/// other id is 8.
pub fn required_java_major(version_id: &str, version: &Version) -> u32 {
    if let Some(java) = &version.java_version {
        return java.major_version;
    }
    major_from_release_id(version_id)
}

const JAVA_SEARCH_DEPTH: u32 = 4;

pub fn java_binary_name(os: OsName) -> &'static str {
    match os {
        OsName::Windows => "java.exe",
        OsName::Linux | OsName::Osx => "java",
    }
}

/// Usual install directories. A missing directory is not an error.
pub fn default_java_roots(os: OsName) -> &'static [&'static str] {
    match os {
        OsName::Windows => &[
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\BellSoft",
            r"C:\Program Files\Zulu",
        ],
        OsName::Linux => &["/usr/lib/jvm", "/usr/java"],
        OsName::Osx => &[
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ],
    }
}

/// Major version printed by `java -version`.
///
/// Java 8 reports `1.8.0_51`. Java 9 and later report `21.0.7`.
pub fn parse_java_version(output: &str) -> Result<u32> {
    for line in output.lines() {
        let Some(version) = quoted_version(line) else {
            continue;
        };
        return major_from_version(version);
    }
    Err(CoreError::JavaProbe)
}

pub fn probe_java_major(executable: &Path) -> Result<u32> {
    if let Some(major) = major_from_release_file(executable) {
        return Ok(major);
    }
    let output = std::process::Command::new(executable)
        .arg("-version")
        .output()?;
    if !output.status.success() {
        return Err(CoreError::JavaProbe);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    parse_java_version(&text)
}

pub fn validate_java(executable: &Path, required: &crate::JavaVersion) -> Result<u32> {
    let found = probe_java_major(executable)?;
    if found != required.major_version {
        return Err(CoreError::JavaMajor {
            found,
            required: required.major_version,
        });
    }
    Ok(found)
}

/// `java` / `java.exe` under each root, including the usual `bin` and macOS `.jdk` layouts.
pub fn discover_javas(roots: &[PathBuf], os: OsName) -> Vec<PathBuf> {
    let name = java_binary_name(os);
    let mut found = Vec::new();
    for root in roots {
        collect_javas(root, name, 0, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaDetections {
    pub java25: Vec<String>,
    pub java21: Vec<String>,
    pub java17: Vec<String>,
    pub java8: Vec<String>,
}

/// Executable already stored under `runtime/` for each major the settings screen offers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaPaths {
    pub java25: Option<String>,
    pub java21: Option<String>,
    pub java17: Option<String>,
    pub java8: Option<String>,
}

/// Whether each path is a file of that major.
///
/// The JDK `release` file answers this without starting a JVM. `java -version`
/// runs only when that file is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaStatus {
    pub java25: bool,
    pub java21: bool,
    pub java17: bool,
    pub java8: bool,
}

/// System install folders plus this launcher's `runtime` directory.
pub fn java_detect_roots(cache: &SharedCache, os: OsName) -> Vec<PathBuf> {
    let mut roots = default_java_roots(os)
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    roots.push(cache.root().join("runtime"));
    roots
}

/// Executables `discover_javas` finds, grouped by the majors the settings screen offers.
pub fn detect_javas(os: OsName) -> JavaDetections {
    let roots = default_java_roots(os)
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    group_detected(&roots, os)
}

/// Same search as [`detect_javas`], and also the Javas this launcher installed.
pub fn detect_known_javas(cache: &SharedCache, os: OsName) -> JavaDetections {
    group_detected(&java_detect_roots(cache, os), os)
}

fn group_detected(roots: &[PathBuf], os: OsName) -> JavaDetections {
    let mut found = JavaDetections::default();
    for path in discover_javas(roots, os) {
        let Ok(major) = probe_java_major(&path) else {
            continue;
        };
        let text = path.display().to_string();
        match major {
            25 => found.java25.push(text),
            21 => found.java21.push(text),
            17 => found.java17.push(text),
            8 => found.java8.push(text),
            _ => {}
        }
    }
    found
}

/// Java already installed in this cache's `runtime` folder. A missing major stays empty.
pub fn runtime_java_paths(
    cache: &SharedCache,
    environment: &LaunchEnvironment,
) -> Result<JavaPaths> {
    Ok(JavaPaths {
        java25: runtime_executable(cache, 25, environment)?,
        java21: runtime_executable(cache, 21, environment)?,
        java17: runtime_executable(cache, 17, environment)?,
        java8: runtime_executable(cache, 8, environment)?,
    })
}

fn runtime_executable(
    cache: &SharedCache,
    major: u32,
    environment: &LaunchEnvironment,
) -> Result<Option<String>> {
    let Some(root) = find_temurin(cache, major, environment)? else {
        return Ok(None);
    };
    let executable = java_executable(&root, environment.os);
    if executable.is_file() {
        Ok(Some(executable.display().to_string()))
    } else {
        Ok(None)
    }
}

/// Folder a file picker should open.
///
/// Uses the directory of `current` when that directory exists. An empty path,
/// or one whose directory is missing, opens `fallback`.
pub fn browse_start_directory(current: &str, fallback: &Path) -> PathBuf {
    let current = current.trim();
    if !current.is_empty() {
        let path = Path::new(current);
        if path.is_dir() {
            return path.to_path_buf();
        }
        if let Some(parent) = path.parent()
            && parent.is_dir()
        {
            return parent.to_path_buf();
        }
    }
    fallback.to_path_buf()
}

pub fn java_installation_status(paths: &JavaPaths) -> JavaStatus {
    JavaStatus {
        java25: java_matches(paths.java25.as_deref(), 25),
        java21: java_matches(paths.java21.as_deref(), 21),
        java17: java_matches(paths.java17.as_deref(), 17),
        java8: java_matches(paths.java8.as_deref(), 8),
    }
}

fn java_matches(path: Option<&str>, major: u32) -> bool {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let path = Path::new(path);
    if !path.is_file() {
        return false;
    }
    probe_java_major(path).ok() == Some(major)
}

/// First executable whose major version equals `required`. Others are skipped.
pub fn matching_java(executables: &[PathBuf], required: &crate::JavaVersion) -> Option<PathBuf> {
    for executable in executables {
        match probe_java_major(executable) {
            Ok(major) if major == required.major_version => return Some(executable.clone()),
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    path = %executable.display(),
                    "installed Java could not be read: {error}"
                );
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeIndex {
    #[serde(flatten)]
    pub platforms: BTreeMap<String, BTreeMap<String, Vec<JavaRuntimeBuild>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeBuild {
    pub availability: JavaRuntimeAvailability,
    pub manifest: JavaRuntimeManifestRef,
    pub version: JavaRuntimeVersionName,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeAvailability {
    pub group: u32,
    pub progress: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeManifestRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeVersionName {
    pub name: String,
    pub released: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeManifest {
    pub files: BTreeMap<String, JavaRuntimeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JavaRuntimeEntry {
    File {
        #[serde(default)]
        executable: bool,
        downloads: JavaRuntimeDownloads,
    },
    Directory,
    Link {
        target: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeDownloads {
    pub raw: JavaRuntimeDownload,
    #[serde(default)]
    pub lzma: Option<JavaRuntimeDownload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JavaRuntimeDownload {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

pub enum JavaChoice<'a> {
    Mojang {
        component: &'a str,
        os: OsName,
        arch: Arch,
        manifest: &'a JavaRuntimeManifest,
    },
    Custom(&'a Path),
}

pub fn parse_java_runtime_index(json: &str) -> Result<JavaRuntimeIndex> {
    serde_json::from_str(json).map_err(CoreError::JavaRuntime)
}

pub fn parse_java_runtime_manifest(json: &str) -> Result<JavaRuntimeManifest> {
    serde_json::from_str(json).map_err(CoreError::JavaRuntime)
}

pub async fn fetch_java_runtime_index(client: &reqwest::Client) -> Result<JavaRuntimeIndex> {
    let body = client
        .get(JAVA_RUNTIME_INDEX_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    parse_java_runtime_index(&body)
}

pub fn runtime_platform(os: OsName, arch: Arch) -> Result<&'static str> {
    match (os, arch) {
        (OsName::Windows, Arch::X86_64) => Ok("windows-x64"),
        (OsName::Windows, Arch::X86) => Ok("windows-x86"),
        (OsName::Linux, Arch::X86_64) => Ok("linux"),
        (OsName::Linux, Arch::X86) => Ok("linux-i386"),
        (OsName::Osx, Arch::X86_64) => Ok("mac-os"),
        (OsName::Osx, Arch::X86) => Err(CoreError::JavaRuntimeMissing),
    }
}

pub fn select_runtime<'a>(
    index: &'a JavaRuntimeIndex,
    os: OsName,
    arch: Arch,
    component: &str,
) -> Result<&'a JavaRuntimeManifestRef> {
    let platform = runtime_platform(os, arch)?;
    let builds = index
        .platforms
        .get(platform)
        .and_then(|components| components.get(component))
        .filter(|builds| !builds.is_empty())
        .ok_or(CoreError::JavaRuntimeMissing)?;
    let ready = builds
        .iter()
        .rev()
        .find(|build| build.availability.progress == 100)
        .or_else(|| builds.last());
    let Some(build) = ready else {
        return Err(CoreError::JavaRuntimeMissing);
    };
    Ok(&build.manifest)
}

pub async fn install_java(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    choice: JavaChoice<'_>,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf> {
    let JavaChoice::Mojang {
        component,
        os,
        arch,
        manifest,
    } = choice
    else {
        let JavaChoice::Custom(executable) = choice else {
            return Err(CoreError::JavaPath);
        };
        if !executable.is_file() {
            return Err(CoreError::JavaPath);
        }
        return Ok(executable.to_path_buf());
    };

    let platform = runtime_platform(os, arch)?;
    let root = cache.runtime_dir(component, platform)?;
    std::fs::create_dir_all(&root)?;

    let mut downloads = Vec::new();
    let mut links = Vec::new();
    for (relative, entry) in &manifest.files {
        let path = under(&root, relative)?;
        match entry {
            JavaRuntimeEntry::Directory => std::fs::create_dir_all(path)?,
            JavaRuntimeEntry::File {
                executable,
                downloads: files,
            } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if !file_matches(&path, &files.raw.sha1)? {
                    downloads.push(DownloadTask {
                        url: files.raw.url.clone(),
                        destination: path.clone(),
                        sha1: files.raw.sha1.clone(),
                        size: files.raw.size,
                    });
                }
                if *executable {
                    links.push(Executable(path));
                }
            }
            JavaRuntimeEntry::Link { target } => links.push(Link {
                path,
                target: target.clone(),
            }),
        }
    }

    download_all(
        client,
        &downloads,
        crate::shared_transfer_limits(),
        download_concurrency(cache, settings)?,
        crate::write_concurrency(cache, settings)?,
        DEFAULT_RETRY,
        progress,
    )
    .await?;

    for item in links {
        match item {
            Executable(path) => mark_executable(&path)?,
            Link { path, target } => place_link(&path, &target)?,
        }
    }

    Ok(java_executable(&root, os))
}

enum Placed {
    Executable(PathBuf),
    Link { path: PathBuf, target: String },
}

use Placed::{Executable, Link};

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
fn temurin_folder_name(
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

fn platform_label(os: OsName, arch: Arch) -> String {
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

fn find_temurin(
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

fn major_from_release_id(version_id: &str) -> u32 {
    let Some(release) = parse_release(version_id) else {
        return 8;
    };
    if release >= (1, 20, 5) {
        return 21;
    }
    if release >= (1, 18, 0) {
        return 17;
    }
    if release >= (1, 17, 0) {
        return 16;
    }
    8
}

fn parse_release(version_id: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version_id.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = match parts.next() {
        Some(part) => part.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
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

fn extract_java_archive(archive: &Path, destination: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.ends_with(".tar.gz") {
        extract_tar_gz(archive, destination)
    } else if name.ends_with(".zip") {
        extract_zip(archive, destination)
    } else {
        Err(CoreError::JavaRuntimeMissing)
    }
}

fn extract_zip(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(std::io::Error::other)?;
        let Some(enclosed) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            return Err(CoreError::CachePath);
        };
        let relative = archive_relative(&enclosed)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let relative = archive_relative(&path)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if kind.is_symlink() {
            let Some(link) = entry.link_name()? else {
                return Err(CoreError::CachePath);
            };
            let Some(link) = link.to_str() else {
                return Err(CoreError::CachePath);
            };
            place_link(&target, link)?;
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

/// Drops the archive's top directory, so `jdk-21/bin/java` lands at `bin/java`.
fn archive_relative(path: &Path) -> Result<PathBuf> {
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Normal(_)) => {}
        _ => return Err(CoreError::CachePath),
    }
    let mut relative = PathBuf::new();
    for component in parts {
        match component {
            Component::Normal(name) => relative.push(name),
            _ => return Err(CoreError::CachePath),
        }
    }
    Ok(relative)
}

fn java_executable(root: &Path, os: OsName) -> PathBuf {
    match os {
        OsName::Windows => root.join("bin").join("java.exe"),
        OsName::Linux | OsName::Osx => root.join("bin").join("java"),
    }
}

fn under(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty() {
        return Err(CoreError::CachePath);
    }
    let mut full = root.to_path_buf();
    for component in relative.split(['/', '\\']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(CoreError::CachePath);
        }
        full.push(component);
    }
    Ok(full)
}

fn file_matches(path: &Path, expected_sha1: &str) -> Result<bool> {
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

fn mark_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        std::fs::set_permissions(path, permissions)?;
    }
    let _ = path;
    Ok(())
}

fn place_link(link: &Path, target: &str) -> Result<()> {
    if link.exists() {
        std::fs::remove_file(link)?;
    }
    let relative = Path::new(target);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(relative, link)?;
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_file(relative, link).is_err() {
            tracing::warn!("Java runtime symlink was copied because the link could not be created");
            let Some(parent) = link.parent() else {
                return Err(CoreError::CachePath);
            };
            std::fs::copy(parent.join(relative), link)?;
        }
    }
    Ok(())
}

fn quoted_version(line: &str) -> Option<&str> {
    let marker = "version \"";
    let start = line.find(marker)? + marker.len();
    let rest = line.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

fn major_from_release_file(executable: &Path) -> Option<u32> {
    let parent = executable.parent()?;
    let home = if parent.file_name().is_some_and(|name| name == "bin") {
        parent.parent()?
    } else {
        parent
    };
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    for line in text.lines() {
        let Some(version) = line.trim().strip_prefix("JAVA_VERSION=") else {
            continue;
        };
        let version = version.trim().trim_matches('"');
        if let Ok(major) = major_from_version(version) {
            return Some(major);
        }
    }
    None
}

fn major_from_version(version: &str) -> Result<u32> {
    let mut parts = version.split('.');
    let Some(first) = parts.next().filter(|part| !part.is_empty()) else {
        return Err(CoreError::JavaProbe);
    };
    let component = if first == "1" {
        parts.next().ok_or(CoreError::JavaProbe)?
    } else {
        first
    };
    let digits: String = component
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return Err(CoreError::JavaProbe);
    }
    digits.parse().map_err(|_| CoreError::JavaProbe)
}

fn collect_javas(dir: &Path, name: &str, depth: u32, found: &mut Vec<PathBuf>) {
    let candidate = dir.join(name);
    if candidate.is_file() {
        found.push(candidate);
    }
    if depth == JAVA_SEARCH_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            tracing::warn!(path = %dir.display(), "Java directory could not be read: {error}");
            return;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            collect_javas(&path, name, depth + 1, found);
        }
    }
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
    use std::io::Write;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use sha2::{Digest as Sha256Digest, Sha256};

    use super::{
        archive_relative, extract_java_archive, hex_encode, install_temurin, major_from_release_id,
        platform_label, required_java_major, temurin_folder_name,
    };
    use crate::{
        Arch, AssetIndex, CoreError, Download, DownloadProgress, GameArguments, JavaVersion,
        LaunchEnvironment, OsName, SharedCache, Version, VersionDownloads,
    };

    #[test]
    fn a_release_id_picks_the_java_that_release_needs() {
        assert_eq!(major_from_release_id("1.16.5"), 8);
        assert_eq!(major_from_release_id("1.5.2"), 8);
        assert_eq!(major_from_release_id("24w14a"), 8);
        assert_eq!(major_from_release_id("1.17"), 16);
        assert_eq!(major_from_release_id("1.17.1"), 16);
        assert_eq!(major_from_release_id("1.18"), 17);
        assert_eq!(major_from_release_id("1.19.4"), 17);
        assert_eq!(major_from_release_id("1.20"), 17);
        assert_eq!(major_from_release_id("1.20.4"), 17);
        assert_eq!(major_from_release_id("1.20.5"), 21);
        assert_eq!(major_from_release_id("1.21.11"), 21);
        assert_eq!(major_from_release_id("26.3"), 21);
    }

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

    #[test]
    fn java_version_in_the_json_wins_over_the_release_id() {
        let version = empty_version(Some(JavaVersion {
            component: "java-runtime-epsilon".to_string(),
            major_version: 25,
        }));
        assert_eq!(required_java_major("1.16.5", &version), 25);
    }

    #[test]
    fn an_archive_path_outside_the_runtime_is_rejected() {
        assert!(archive_relative(std::path::Path::new("jdk-21/../outside")).is_err());
    }

    #[test]
    fn a_gzip_archive_drops_its_top_directory() {
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        let archive = directory.join("temurin.tar.gz");
        std::fs::write(&archive, sample_tar_gz()).expect("archive");
        extract_java_archive(&archive, &directory).expect("extract");
        assert_eq!(
            std::fs::read(directory.join("bin").join("java")).expect("java"),
            b"temurin"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn temurin_is_stored_under_its_major_version() {
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

    fn empty_version(java: Option<JavaVersion>) -> Version {
        Version {
            arguments: GameArguments::Legacy(String::new()),
            libraries: Vec::new(),
            asset_index: AssetIndex {
                id: "t".to_string(),
                sha1: "a".repeat(40),
                size: 1,
                total_size: 1,
                url: "https://example.invalid".to_string(),
            },
            main_class: "a.B".to_string(),
            downloads: VersionDownloads {
                client: Download {
                    sha1: "b".repeat(40),
                    size: 1,
                    url: "https://example.invalid".to_string(),
                },
                server: None,
            },
            java_version: java,
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

    fn sample_tar_gz() -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_path("jdk-21/bin/java").expect("path");
        header.set_size(7);
        header.set_mode(0o755);
        header.set_cksum();
        let mut builder = tar::Builder::new(Vec::new());
        builder.append(&header, &b"temurin"[..]).expect("tar entry");
        let tar_bytes = builder.into_inner().expect("tar");
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar_bytes).expect("gzip");
        encoder.finish().expect("gzip finish")
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
