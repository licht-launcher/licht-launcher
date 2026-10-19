use std::path::{Path, PathBuf};

use crate::hashes::{file_matches_sha1, sha1_hex};
use crate::natives::{classifier_matches, native_classifier};
use crate::{
    ADOPTIUM_API, AssetIndexFile, CoreError, DEFAULT_RETRY, DownloadComponent, DownloadOptions,
    DownloadPhase, DownloadTask, InstallationContext, LaunchEnvironment, Result, SharedCache,
    Version, applicable_libraries, download_concurrency, install_temurin, parse_asset_index,
    parse_version, required_java_major, use_custom_java,
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
    progress: impl Into<InstallationContext>,
) -> Result<()> {
    let progress = progress.into();
    cache.create()?;
    write_asset_index(cache, plan.version, plan.asset_index_json)?;

    let mut pending = Vec::new();
    for task in download_tasks(cache, &plan)? {
        if file_matches_sha1(&task.destination, &task.sha1)? {
            continue;
        }
        if let Some(parent) = task.destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        pending.push(task);
    }

    progress
        .queue(
            client,
            &pending,
            DownloadOptions {
                limits: crate::shared_transfer_limits(),
                download_concurrency: download_concurrency(cache, settings)?,
                write_concurrency: crate::write_concurrency(cache, settings)?,
                retry: DEFAULT_RETRY,
            },
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
    progress: impl Into<InstallationContext>,
) -> Result<PathBuf> {
    let progress = progress.into().component(DownloadComponent::Minecraft);
    progress.stage(DownloadPhase::Preparing).await?;
    let json = progress
        .metadata(read_text(client, request.version_json_url))
        .await?;
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

    let index_json = progress
        .metadata(read_text(client, &version.asset_index.url))
        .await?;
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

/// Installs a vanilla version or an existing inherited loader profile through the same pipeline.
pub async fn install_game_version(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    id: &str,
    environment: &LaunchEnvironment,
    progress: impl Into<InstallationContext>,
) -> Result<PathBuf> {
    let progress = progress.into();
    install_requested(
        client,
        cache,
        settings,
        id,
        environment,
        progress,
        Vec::new(),
    )
    .await
}

async fn install_requested(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    id: &str,
    environment: &LaunchEnvironment,
    progress: InstallationContext,
    mut chain: Vec<String>,
) -> Result<PathBuf> {
    cache.version_json(id)?;
    if chain.len() >= 8 || chain.iter().any(|version| version == id) {
        return Err(CoreError::LoaderInstall {
            detail: "cyclic or excessively deep version inheritance".to_string(),
        });
    }
    chain.push(id.to_string());
    if let Some(parent) = crate::local_inherits_from(cache, id)? {
        Box::pin(install_requested(
            client,
            cache,
            settings,
            &parent,
            environment,
            progress.clone(),
            chain,
        ))
        .await?;
        return install_local_version(client, cache, settings, id, environment, progress).await;
    }
    progress.stage(DownloadPhase::Preparing).await?;
    let manifest = progress
        .metadata(crate::fetch_version_manifest(client, cache))
        .await;
    progress.checkpoint().await?;
    if let Ok(manifest) = &manifest
        && let Some(entry) = manifest.versions.iter().find(|entry| entry.id == id)
    {
        return install_game(
            client,
            cache,
            settings,
            GameInstall {
                version_id: id,
                version_json_url: &entry.url,
                version_sha1: &entry.sha1,
                environment,
                asset_base: ASSET_OBJECT_BASE,
                java: None,
            },
            progress,
        )
        .await;
    }
    if crate::local_version_exists(cache, id) {
        return install_local_version(client, cache, settings, id, environment, progress).await;
    }
    match manifest {
        Err(error) => Err(error),
        Ok(_) => Err(CoreError::VersionMissing),
    }
}

async fn install_local_version(
    client: &reqwest::Client,
    cache: &SharedCache,
    settings: &SharedCache,
    id: &str,
    environment: &LaunchEnvironment,
    progress: InstallationContext,
) -> Result<PathBuf> {
    let component = if id.starts_with("fabric-loader") {
        DownloadComponent::Fabric
    } else if id.starts_with("quilt-loader") {
        DownloadComponent::Quilt
    } else if id.starts_with("neoforge") {
        DownloadComponent::NeoForge
    } else if id.starts_with("forge") {
        DownloadComponent::Forge
    } else {
        DownloadComponent::Minecraft
    };
    let progress = progress.component(component);
    progress.stage(DownloadPhase::Preparing).await?;
    cache.create()?;
    let cache = installation_cache(cache)?;
    let is_forge = crate::loaders::forge::request(&cache, id)?.is_some();
    let _loader_guard = if is_forge {
        Some(tokio::select! {
            _ = progress.control.cancelled() => return Err(CoreError::DownloadCancelled),
            guard = crate::loaders::forge::INSTALL_LOCK.lock() => guard,
        })
    } else {
        None
    };
    let version = parse_version(&crate::load_resolved_version_json(&cache, id)?)?;
    let major = required_java_major(id, &version);
    let configured = crate::load_settings(settings)?;
    let java = match crate::configured_java(&configured, major) {
        Some(java) => {
            let found = crate::probe_java_major(&java)?;
            if found != major {
                return Err(CoreError::JavaMajor {
                    required: major,
                    found,
                });
            }
            java
        }
        None => {
            install_temurin(
                client,
                &cache,
                major,
                environment,
                ADOPTIUM_API,
                progress.clone(),
            )
            .await?
        }
    };
    let loader_files = crate::loaders::forge::prepare(
        client,
        &cache,
        settings,
        id,
        &java,
        environment,
        progress.clone(),
    )
    .await?;
    let version = parse_version(&crate::load_resolved_version_json(&cache, id)?)?;
    let index_path = cache.asset_index(&version.asset_index.id)?;
    let index_json = if file_matches_sha1(&index_path, &version.asset_index.sha1)? {
        std::fs::read_to_string(index_path)?
    } else {
        progress
            .metadata(read_text(client, &version.asset_index.url))
            .await?
    };
    let assets = parse_asset_index(&index_json)?;
    install_version(
        client,
        &cache,
        settings,
        InstallPlan {
            version_id: id,
            version: &version,
            asset_index_json: &index_json,
            assets: &assets,
            environment,
            asset_base: ASSET_OBJECT_BASE,
        },
        progress,
    )
    .await?;
    if let Some(files) = loader_files {
        crate::loaders::forge::complete(&cache, id, &files)?;
    }
    Ok(java)
}

fn installation_cache(cache: &SharedCache) -> Result<SharedCache> {
    // Java's Windows classpath reader does not accept canonicalize's verbatim prefix.
    Ok(SharedCache::at(std::path::absolute(cache.root())?))
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

fn write_asset_index(cache: &SharedCache, version: &Version, json: &str) -> Result<()> {
    let expected = &version.asset_index.sha1;
    let actual = sha1_hex(json.as_bytes());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(CoreError::Sha1Mismatch {
            expected: expected.clone(),
            actual,
        });
    }

    let path = cache.asset_index(&version.asset_index.id)?;
    if file_matches_sha1(&path, expected)? {
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

#[cfg(test)]
mod loader_path_tests {
    #[test]
    fn installation_paths_are_absolute_and_usable_by_java() {
        let cache = crate::SharedCache::at(std::env::temp_dir().join("licht loader Java paths"));
        let resolved = super::installation_cache(&cache).expect("absolute path");
        assert!(resolved.root().is_absolute());
        #[cfg(windows)]
        assert!(!resolved.root().display().to_string().starts_with(r"\\?\"));
    }
}
