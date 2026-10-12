use serde::Deserialize;

use crate::{Arch, CoreError, OsName, Result};

pub(super) struct TemurinRelease {
    pub image: String,
    pub version: String,
    pub package: TemurinPackage,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct TemurinPackage {
    pub checksum: String,
    pub link: String,
    pub name: String,
    pub size: u64,
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

pub(super) async fn temurin_package(
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
