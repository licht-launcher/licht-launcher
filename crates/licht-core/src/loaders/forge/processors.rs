use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Client;
use tokio::process::Command;

use super::archive::{Installer, Processor, entry};
use super::artifacts::{failure, maven_path, write_verified};
use crate::hashes::file_matches_sha1;
use crate::{CoreError, DEFAULT_RETRY, OsName, Result, SharedCache};

pub(super) fn expand(value: &str, data: &BTreeMap<String, String>) -> Result<String> {
    let mut expanded = String::new();
    let mut rest = value;
    while let Some((prefix, token)) = rest.split_once('{') {
        expanded.push_str(prefix);
        let (key, after) = token
            .split_once('}')
            .ok_or_else(|| failure("unterminated processor token"))?;
        expanded.push_str(
            data.get(key)
                .ok_or_else(|| failure("unknown processor token"))?,
        );
        rest = after;
    }
    expanded.push_str(rest);
    Ok(expanded)
}

fn resolve(value: &str, data: &BTreeMap<String, String>, cache: &SharedCache) -> Result<String> {
    if let Some(coordinate) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return Ok(cache
            .library(&maven_path(coordinate)?)?
            .display()
            .to_string());
    }
    expand(value, data)
}

fn output_path(
    value: &str,
    data: &BTreeMap<String, String>,
    cache: &SharedCache,
) -> Result<PathBuf> {
    let path = PathBuf::from(resolve(value, data, cache)?);
    let root = cache.root().join("libraries");
    let relative = path
        .strip_prefix(root)
        .map_err(|_| failure("processor output is outside the library cache"))?;
    let relative = relative
        .to_str()
        .ok_or(CoreError::CachePath)?
        .replace('\\', "/");
    cache.library(&relative)
}

pub(super) fn command(
    java: &Path,
    processor: &Processor,
    data: &BTreeMap<String, String>,
    cache: &SharedCache,
    os: OsName,
) -> Result<Vec<String>> {
    let jar = cache.library(&maven_path(&processor.jar)?)?;
    let mut archive =
        zip::ZipArchive::new(fs::File::open(&jar)?).map_err(CoreError::LoaderArchive)?;
    let mut manifest = String::new();
    archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(CoreError::LoaderArchive)?
        .read_to_string(&mut manifest)?;
    let manifest = manifest.replace("\r\n ", "").replace("\n ", "");
    let main = manifest
        .lines()
        .find_map(|line| line.strip_prefix("Main-Class: "))
        .map(str::trim)
        .ok_or_else(|| failure("processor JAR has no Main-Class"))?;
    if main.is_empty()
        || !main
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._$".contains(&byte))
    {
        return Err(failure("invalid processor Main-Class"));
    }
    let mut classpath = vec![jar.display().to_string()];
    for library in &processor.classpath {
        let path = cache.library(&maven_path(library)?)?;
        if !path.is_file() {
            return Err(failure("processor classpath library is missing"));
        }
        classpath.push(path.display().to_string());
    }
    let separator = if os == OsName::Windows { ";" } else { ":" };
    let mut command = vec![
        java.display().to_string(),
        "-Djava.awt.headless=true".to_string(),
        "-cp".to_string(),
        classpath.join(separator),
        main.to_string(),
    ];
    for argument in &processor.args {
        command.push(resolve(argument, data, cache)?);
    }
    Ok(command)
}

pub(super) async fn run(
    client: &Client,
    installer: &mut Installer,
    cache: &SharedCache,
    java: &Path,
    installer_path: &Path,
    os: OsName,
) -> Result<Vec<PathBuf>> {
    let work = installer_path
        .parent()
        .ok_or(CoreError::CachePath)?
        .join("processor-data");
    fs::create_dir_all(&work)?;
    let mut data = BTreeMap::from([
        ("SIDE".to_string(), "client".to_string()),
        ("ROOT".to_string(), cache.root().display().to_string()),
        (
            "LIBRARY_DIR".to_string(),
            cache.root().join("libraries").display().to_string(),
        ),
        (
            "INSTALLER".to_string(),
            installer_path.display().to_string(),
        ),
        (
            "MINECRAFT_VERSION".to_string(),
            installer.profile.minecraft.clone(),
        ),
        (
            "MINECRAFT_JAR".to_string(),
            cache
                .version_jar(&installer.profile.minecraft)?
                .display()
                .to_string(),
        ),
    ]);
    for (key, side) in &installer.profile.data {
        let Some(value) = &side.client else {
            continue;
        };
        let resolved = if let Some(literal) = value
            .strip_prefix('\'')
            .and_then(|value| value.strip_suffix('\''))
        {
            literal.to_string()
        } else if value.starts_with('[') {
            resolve(value, &data, cache)?
        } else {
            let bytes = entry(&mut installer.archive, value)?;
            let relative = value.strip_prefix('/').unwrap_or(value);
            let destination = work.join(relative);
            let hash = crate::hashes::sha1_hex(&bytes);
            write_verified(&destination, &bytes, &hash)?;
            destination.display().to_string()
        };
        data.insert(key.clone(), resolved);
    }
    let mut produced = Vec::new();
    for (index, processor) in installer
        .profile
        .processors
        .iter()
        .enumerate()
        .filter(|(_, processor)| processor.is_client())
    {
        let outputs = processor
            .outputs
            .iter()
            .map(|(path, hash)| Ok((output_path(path, &data, cache)?, expand(hash, &data)?)))
            .collect::<Result<Vec<_>>>()?;
        let mut required = Vec::new();
        for pair in processor.args.windows(2) {
            if ["--output", "--slim", "--extra"].contains(&pair[0].as_str()) {
                required.push(output_path(&pair[1], &data, cache)?);
            }
        }
        produced.extend(required.clone());
        produced.extend(outputs.iter().map(|(path, _)| path.clone()));
        if !outputs.is_empty() {
            let valid = outputs
                .iter()
                .map(|(path, hash)| file_matches_sha1(path, hash))
                .collect::<Result<Vec<_>>>()?;
            if valid.into_iter().all(|valid| valid) {
                continue;
            }
        }
        if processor.args.iter().any(|arg| arg == "DOWNLOAD_MOJMAPS") {
            let output = required
                .first()
                .ok_or_else(|| failure("mapping processor has no output"))?;
            let json: serde_json::Value = serde_json::from_slice(&fs::read(
                cache.version_json(&installer.profile.minecraft)?,
            )?)
            .map_err(CoreError::LoaderJson)?;
            let mapping = json
                .pointer("/downloads/client_mappings")
                .ok_or_else(|| failure("Mojang client mappings are missing"))?;
            let sha1 = mapping
                .get("sha1")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| failure("Mojang mapping SHA1 is missing"))?;
            let url = mapping
                .get("url")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| failure("Mojang mapping URL is missing"))?;
            crate::download_file(client, url, output, sha1, DEFAULT_RETRY).await?;
        } else {
            let arguments = command(java, processor, &data, cache, os)?;
            tracing::info!(processor = index + 1, jar = %processor.jar, "Running loader client processor");
            let mut child = Command::new(&arguments[0]);
            child
                .args(&arguments[1..])
                .current_dir(cache.root())
                .kill_on_drop(true);
            let result = tokio::time::timeout(Duration::from_secs(600), child.output())
                .await
                .map_err(|_| failure("client processor timed out"))??;
            if !result.status.success() {
                return Err(CoreError::LoaderProcessor {
                    processor: index + 1,
                    code: result.status.code(),
                });
            }
        }
        for path in required {
            if !path.is_file() {
                return Err(failure("client processor did not produce its output"));
            }
        }
        for (path, expected) in outputs {
            if !file_matches_sha1(&path, &expected)? {
                return Err(CoreError::Sha1Mismatch {
                    expected,
                    actual: crate::hashes::sha1_hex(&fs::read(path)?),
                });
            }
        }
    }
    // Some installer generations put final output hashes in `data`, not `outputs`.
    for (key, expected) in &data {
        if let Some(output) = key.strip_suffix("_SHA").and_then(|name| data.get(name)) {
            let path = output_path(output, &data, cache)?;
            if !file_matches_sha1(&path, expected)? {
                return Err(failure("generated library SHA1 does not match"));
            }
            produced.push(path);
        }
    }
    Ok(produced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_expanded_without_shell_splitting_and_unknown_names_fail() {
        let data = BTreeMap::from([
            ("ROOT".to_string(), "C:\\Licht data".to_string()),
            ("SIDE".to_string(), "client".to_string()),
        ]);
        assert_eq!(
            expand("{ROOT}/libraries/{SIDE}", &data).expect("expand"),
            "C:\\Licht data/libraries/client"
        );
        assert!(expand("{MISSING}", &data).is_err());
        assert!(expand("{ROOT", &data).is_err());
    }
}
