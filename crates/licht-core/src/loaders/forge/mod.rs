mod archive;
mod artifacts;
mod metadata;
mod processors;
mod receipt;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, mpsc};

use crate::hashes::file_matches_sha1;
use crate::{
    CoreError, DEFAULT_RETRY, DownloadProgress, LaunchEnvironment, LoaderChannel, Result,
    SharedCache,
};
use archive::Installer;
use artifacts::{checksum, failure, normalize_libraries};
pub use metadata::{FORGE_METADATA, NEOFORGE_METADATA, list_forge_like_versions};

// Different loader builds can generate the same intermediate Minecraft libraries.
pub(crate) static INSTALL_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForgeFamily {
    Forge,
    NeoForge,
}

impl ForgeFamily {
    fn prefix(self) -> &'static str {
        match self {
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
        }
    }
    fn metadata(self) -> &'static str {
        match self {
            Self::Forge => FORGE_METADATA,
            Self::NeoForge => NEOFORGE_METADATA,
        }
    }
    fn installer_url(self, build: &str, game_version: &str) -> String {
        match self {
            Self::Forge => {
                // .mrpack dependencies carry only the Forge build; the wizard carries MC-build.
                let build = if build.starts_with(&format!("{game_version}-")) {
                    build.to_string()
                } else {
                    format!("{game_version}-{build}")
                };
                format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{build}/forge-{build}-installer.jar"
                )
            }
            Self::NeoForge => format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{build}/neoforge-{build}-installer.jar"
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForgeRequest {
    family: ForgeFamily,
    build: String,
    game_version: String,
}

impl ForgeRequest {
    fn validate(&self) -> Result<()> {
        if [&self.build, &self.game_version].iter().any(|value| {
            value.is_empty()
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
        }) {
            return Err(CoreError::LoaderVersionMissing);
        }
        let matches = match self.family {
            ForgeFamily::Forge => true,
            ForgeFamily::NeoForge => self.build.starts_with(&format!(
                "{}.",
                self.game_version
                    .strip_prefix("1.")
                    .unwrap_or(&self.game_version)
            )),
        };
        if !matches {
            return Err(failure("loader build does not match the Minecraft version"));
        }
        Ok(())
    }
}

pub(crate) fn request(cache: &SharedCache, id: &str) -> Result<Option<ForgeRequest>> {
    let path = cache.version_json(id)?;
    if !path.is_file() {
        return Ok(None);
    }
    let version: Value = serde_json::from_slice(&fs::read(path)?).map_err(CoreError::LoaderJson)?;
    let Some(value) = version.get("lichtLoader") else {
        return Ok(None);
    };
    if !matches!(
        value.get("family").and_then(Value::as_str),
        Some("forge" | "neoforge")
    ) {
        return Ok(None);
    }
    let request: ForgeRequest =
        serde_json::from_value(value.clone()).map_err(CoreError::LoaderJson)?;
    request.validate()?;
    Ok(Some(request))
}

pub(crate) fn installation_ready(cache: &SharedCache, id: &str) -> bool {
    receipt::ready(cache, id)
}

async fn installer(
    client: &Client,
    cache: &SharedCache,
    id: &str,
    request: &ForgeRequest,
) -> Result<(PathBuf, Installer)> {
    request.validate()?;
    let path = cache
        .version_json(id)?
        .parent()
        .ok_or(CoreError::CachePath)?
        .join("installer.jar");
    let sidecar = path.with_extension("sha1");
    let stored = if sidecar.is_file() {
        Some(fs::read_to_string(&sidecar)?)
    } else {
        None
    };
    if let Some(hash) = stored
        .as_deref()
        .filter(|hash| artifacts::valid_sha1(hash.trim()))
        && file_matches_sha1(&path, hash.trim())?
    {
        let installer = archive::read_installer(&path)?;
        return Ok((path, installer));
    }
    let url = request
        .family
        .installer_url(&request.build, &request.game_version);
    let hash = checksum(client, &format!("{url}.sha1")).await?;
    fs::create_dir_all(path.parent().ok_or(CoreError::CachePath)?)?;
    crate::download_file(client, &url, &path, &hash, DEFAULT_RETRY).await?;
    receipt::atomic_write(&sidecar, hash.as_bytes())?;
    let installer = archive::read_installer(&path)?;
    if installer.profile.minecraft != request.game_version {
        return Err(failure("installer targets a different Minecraft version"));
    }
    Ok((path, installer))
}

async fn write_profile(
    client: &Client,
    cache: &SharedCache,
    id: &str,
    request: &ForgeRequest,
    installer: &mut Installer,
) -> Result<Vec<crate::Artifact>> {
    if installer.profile.minecraft != request.game_version {
        return Err(failure("installer targets a different Minecraft version"));
    }
    let mut libraries = installer
        .version
        .get("libraries")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| failure("loader profile has no libraries"))?;
    let artifacts = normalize_libraries(client, &mut libraries, installer, cache).await?;
    installer.version["libraries"] = Value::Array(libraries);
    installer.version["id"] = Value::String(id.to_string());
    installer.version["inheritsFrom"] = Value::String(request.game_version.clone());
    installer.version["lichtLoader"] =
        serde_json::to_value(request).map_err(CoreError::LoaderJson)?;
    let main = installer
        .version
        .get("mainClass")
        .and_then(Value::as_str)
        .ok_or_else(|| failure("loader main class is missing"))?;
    if main == "net.minecraft.client.main.Main" {
        return Err(failure("installer contains a vanilla placeholder"));
    }
    receipt::atomic_write(
        &cache.version_json(id)?,
        &serde_json::to_vec_pretty(&installer.version).map_err(CoreError::LoaderJson)?,
    )?;
    Ok(artifacts)
}

/// Reads the genuine official profile at creation; client processors run at Install/Repair.
pub async fn install_forge_like(
    client: &Client,
    cache: &SharedCache,
    family: ForgeFamily,
    game_version: &str,
    channel: LoaderChannel,
    loader_version: Option<&str>,
) -> Result<String> {
    let build = match loader_version.filter(|value| !value.is_empty()) {
        Some(build) => build.to_string(),
        None => list_forge_like_versions(client, family.metadata(), game_version, channel)
            .await?
            .into_iter()
            .next()
            .ok_or(CoreError::LoaderVersionMissing)?,
    };
    let request = ForgeRequest {
        family,
        build,
        game_version: game_version.to_string(),
    };
    request.validate()?;
    let id = format!("{}-{}", family.prefix(), request.build);
    let _guard = INSTALL_LOCK.lock().await;
    if installation_ready(cache, &id) {
        return Ok(id);
    }
    cache.create()?;
    let (_, mut installer) = installer(client, cache, &id, &request).await?;
    write_profile(client, cache, &id, &request, &mut installer).await?;
    Ok(id)
}

pub(crate) async fn prepare(
    client: &Client,
    cache: &SharedCache,
    settings: &SharedCache,
    id: &str,
    java: &Path,
    environment: &LaunchEnvironment,
    progress: mpsc::Sender<DownloadProgress>,
) -> Result<Option<Vec<PathBuf>>> {
    let Some(request) = request(cache, id)? else {
        return Ok(None);
    };
    if installation_ready(cache, id) {
        return Ok(None);
    }
    let (installer_path, mut installer) = installer(client, cache, id, &request).await?;
    let mut artifacts = write_profile(client, cache, id, &request, &mut installer).await?;
    let mut processor_libraries = installer.profile.libraries.clone();
    artifacts.extend(
        normalize_libraries(client, &mut processor_libraries, &mut installer, cache).await?,
    );
    artifacts::download_libraries(client, cache, settings, &artifacts, progress.clone()).await?;
    progress
        .send(DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        })
        .await
        .map_err(|_| failure("installation progress receiver closed"))?;
    let mut files = artifacts
        .iter()
        .map(|artifact| cache.library(&artifact.path))
        .collect::<Result<Vec<_>>>()?;
    files.extend(
        processors::run(
            client,
            &mut installer,
            cache,
            java,
            &installer_path,
            environment.os,
        )
        .await?,
    );
    Ok(Some(files))
}

pub(crate) fn complete(cache: &SharedCache, id: &str, files: &[PathBuf]) -> Result<()> {
    receipt::record(cache, id, files)
}
