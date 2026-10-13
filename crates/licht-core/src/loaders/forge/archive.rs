use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;
use zip::ZipArchive;

use crate::{CoreError, Result};

#[derive(Debug, Deserialize)]
pub(super) struct SideData {
    #[serde(default)]
    pub client: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Processor {
    #[serde(default)]
    pub sides: Vec<String>,
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
}

impl Processor {
    pub fn is_client(&self) -> bool {
        self.sides.is_empty() || self.sides.iter().any(|side| side == "client")
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct Profile {
    pub minecraft: String,
    #[serde(default)]
    pub data: BTreeMap<String, SideData>,
    #[serde(default)]
    pub processors: Vec<Processor>,
    #[serde(default)]
    pub libraries: Vec<Value>,
}

pub(super) struct Installer {
    pub profile: Profile,
    pub version: Value,
    pub archive: ZipArchive<File>,
}

pub(super) fn read_installer(path: &Path) -> Result<Installer> {
    let mut archive = ZipArchive::new(File::open(path)?).map_err(CoreError::LoaderArchive)?;
    let mut profile: Value = serde_json::from_slice(&entry(&mut archive, "install_profile.json")?)
        .map_err(CoreError::LoaderJson)?;
    let version = if let Some(version) = profile.get("versionInfo") {
        version.clone()
    } else {
        let json = profile
            .get("json")
            .and_then(Value::as_str)
            .ok_or(CoreError::LoaderVersionMissing)?;
        serde_json::from_slice(&entry(&mut archive, json)?).map_err(CoreError::LoaderJson)?
    };
    // Older installers store the game id below `install` and embed `versionInfo`.
    if profile.get("minecraft").is_none() {
        let minecraft = profile
            .pointer("/install/minecraft")
            .cloned()
            .ok_or(CoreError::LoaderVersionMissing)?;
        profile["minecraft"] = minecraft;
    }
    let profile = serde_json::from_value(profile).map_err(CoreError::LoaderJson)?;
    Ok(Installer {
        profile,
        version,
        archive,
    })
}

pub(super) fn entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let name = name.strip_prefix('/').unwrap_or(name);
    if name.is_empty()
        || name.contains('\\')
        || name
            .split('/')
            .any(|part| !crate::path_component::path_component_safe(part))
    {
        return Err(CoreError::CachePath);
    }
    let mut file = archive.by_name(name).map_err(CoreError::LoaderArchive)?;
    if file.is_dir()
        || file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err(CoreError::CachePath);
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_profiles_cover_legacy_forge_modern_forge_and_neoforge() {
        for (body, game, count) in [
            (
                include_str!("../../../tests/fixtures/loaders/forge-legacy-install_profile.json"),
                "1.12.2",
                0,
            ),
            (
                include_str!("../../../tests/fixtures/loaders/forge-modern-install_profile.json"),
                "1.20.1",
                6,
            ),
            (
                include_str!("../../../tests/fixtures/loaders/neoforge-install_profile.json"),
                "1.21.1",
                6,
            ),
        ] {
            let profile: Profile = serde_json::from_str(body).expect("official fixture");
            assert_eq!(profile.minecraft, game);
            assert_eq!(
                profile.processors.iter().filter(|p| p.is_client()).count(),
                count
            );
        }
    }
}
