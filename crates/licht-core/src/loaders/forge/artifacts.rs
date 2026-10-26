use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use reqwest::Client;
use serde_json::Value;

use super::archive::{Installer, entry};
use crate::hashes::{file_matches_sha1, sha1_hex};
use crate::{Artifact, CoreError, DEFAULT_RETRY, DownloadTask, Result, SharedCache};

pub(super) fn failure(detail: &str) -> CoreError {
    CoreError::LoaderInstall {
        detail: detail.to_string(),
    }
}

pub(super) fn maven_path(coordinate: &str) -> Result<String> {
    let (coordinate, extension) = coordinate.split_once('@').unwrap_or((coordinate, "jar"));
    let parts: Vec<_> = coordinate.split(':').collect();
    if !(3..=4).contains(&parts.len())
        || !crate::path_component::path_component_safe(extension)
        || extension.contains(['/', '\\'])
        || parts.iter().any(|part| {
            part.contains(['/', '\\']) || !crate::path_component::path_component_safe(part)
        })
        || parts[0]
            .split('.')
            .any(|part| !crate::path_component::path_component_safe(part))
    {
        return Err(CoreError::CachePath);
    }
    let classifier = parts
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    Ok(format!(
        "{}/{}/{}/{}-{}{classifier}.{extension}",
        parts[0].replace('.', "/"),
        parts[1],
        parts[2],
        parts[1],
        parts[2]
    ))
}

pub(super) fn valid_sha1(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) async fn checksum(client: &Client, url: &str) -> Result<String> {
    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let hash = body
        .split_whitespace()
        .next()
        .ok_or_else(|| failure("missing SHA1"))?;
    if !valid_sha1(hash) {
        return Err(failure("invalid SHA1"));
    }
    Ok(hash.to_ascii_lowercase())
}

fn artifact(value: &Value) -> Result<Artifact> {
    if let Some(artifact) = value.pointer("/downloads/artifact") {
        return serde_json::from_value(artifact.clone()).map_err(CoreError::LoaderJson);
    }
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| failure("library has no Maven coordinate"))?;
    let path = maven_path(name)?;
    let base = value
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("https://libraries.minecraft.net/");
    Ok(Artifact {
        url: format!(
            "{}{path}",
            if base.ends_with('/') {
                base.to_string()
            } else {
                format!("{base}/")
            }
        ),
        path,
        sha1: value
            .get("sha1")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        size: value.get("size").and_then(Value::as_u64).unwrap_or(0),
    })
}

pub(super) async fn normalize_libraries(
    client: &Client,
    values: &mut [Value],
    installer: &mut Installer,
    cache: &SharedCache,
) -> Result<Vec<Artifact>> {
    let mut artifacts = Vec::new();
    for value in values {
        let mut artifact = artifact(value)?;
        let destination = cache.library(&artifact.path)?;
        if artifact.url.is_empty() {
            let bytes = entry(&mut installer.archive, &format!("maven/{}", artifact.path))?;
            if artifact.sha1.is_empty() {
                artifact.sha1 = sha1_hex(&bytes);
            }
            write_verified(&destination, &bytes, &artifact.sha1)?;
        } else {
            let url = url::Url::parse(&artifact.url).map_err(|_| failure("invalid library URL"))?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(failure("library URL must use HTTPS without credentials"));
            }
            if artifact.sha1.is_empty() {
                artifact.sha1 = checksum(client, &format!("{}.sha1", artifact.url)).await?;
            }
        }
        if !valid_sha1(&artifact.sha1) {
            return Err(failure("library SHA1 is invalid"));
        }
        value["downloads"] = serde_json::json!({"artifact": {"path": artifact.path, "url": artifact.url, "sha1": artifact.sha1, "size": artifact.size}});
        artifacts.push(artifact);
    }
    Ok(artifacts)
}

pub(super) fn write_verified(destination: &Path, bytes: &[u8], expected: &str) -> Result<()> {
    let actual = sha1_hex(bytes);
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(CoreError::Sha1Mismatch {
            expected: expected.to_string(),
            actual,
        });
    }
    if file_matches_sha1(destination, expected)? {
        return Ok(());
    }
    let parent = destination.parent().ok_or(CoreError::CachePath)?;
    fs::create_dir_all(parent)?;
    let temporary = destination.with_extension("part");
    fs::write(&temporary, bytes)?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)?;
    Ok(())
}

pub(super) async fn download_libraries(
    client: &Client,
    cache: &SharedCache,
    settings: &SharedCache,
    artifacts: &[Artifact],
    progress: impl Into<crate::InstallationContext>,
) -> Result<()> {
    let progress = progress.into();
    let mut pending = BTreeMap::new();
    for artifact in artifacts {
        let destination = cache.library(&artifact.path)?;
        if file_matches_sha1(&destination, &artifact.sha1)? {
            continue;
        }
        if artifact.url.is_empty() {
            return Err(failure("embedded library is missing"));
        }
        pending.insert(
            artifact.path.clone(),
            DownloadTask {
                url: artifact.url.clone(),
                destination,
                sha1: artifact.sha1.clone(),
                size: artifact.size,
            },
        );
    }
    progress
        .queue(
            client,
            &pending.into_values().collect::<Vec<_>>(),
            crate::DownloadOptions {
                limits: crate::settings::configured_transfer_limits(cache, settings)?,
                download_concurrency: crate::download_concurrency(cache, settings)?,
                write_concurrency: crate::write_concurrency(cache, settings)?,
                retry: DEFAULT_RETRY,
            },
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_support_classifiers_and_non_jar_extensions() {
        assert_eq!(
            maven_path("net.neoforged:neoform:1.21.1:mappings@txt").expect("path"),
            "net/neoforged/neoform/1.21.1/neoform-1.21.1-mappings.txt"
        );
        assert_eq!(
            maven_path("a:b:C:bad").expect("four-part coordinate"),
            "a/b/C/b-C-bad.jar"
        );
        for coordinate in [
            "a:b:..",
            r"a:b:C:\bad",
            "a:b:1@../zip",
            "a/..:b:1",
            "a:b:1:CON",
        ] {
            assert!(maven_path(coordinate).is_err());
        }
    }
}
