use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::sync::mpsc;

use super::archive::{java_executable, mark_executable, place_link, under};
use crate::hashes::file_matches_sha1;
use crate::{
    Arch, CoreError, DEFAULT_RETRY, DownloadProgress, DownloadTask, OsName, Result, SharedCache,
    download_all, download_concurrency,
};

pub const JAVA_RUNTIME_INDEX_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

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
                if !file_matches_sha1(&path, &files.raw.sha1)? {
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
