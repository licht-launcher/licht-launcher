use licht_core::{
    ADOPTIUM_API, ASSET_OBJECT_BASE, CoreError, DownloadProgress, GameInstall, SharedCache,
    VersionSummary, fetch_version_manifest, install_game, install_temurin, is_version_ready,
    list_version_summaries, load_instance, load_resolved_version_json, local_inherits_from,
    local_version_exists, parse_version, required_java_major,
};
use tauri::{AppHandle, State};
use tokio::sync::mpsc;

use crate::progress::ProgressThrottle;

use super::install_progress::emit_install_progress;
use super::modpacks::install_modpack_files;
use super::{LauncherPaths, failure, host_environment, http_client};

#[tauri::command]
pub async fn list_versions(paths: State<'_, LauncherPaths>) -> Result<Vec<VersionSummary>, String> {
    let cache = paths.game.clone();
    let client = http_client().map_err(failure)?;
    list_version_summaries(&client, &cache)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn is_version_ready_cmd(
    paths: State<'_, LauncherPaths>,
    version_id: String,
) -> Result<bool, String> {
    Ok(is_version_ready(&paths.game, &version_id))
}

#[tauri::command]
pub async fn install_version(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    version_id: String,
) -> Result<String, String> {
    install_one(&app, &paths.game, &paths.settings, &version_id, &version_id)
        .await
        .map_err(failure)
}

/// Re-downloads game assets and modpack files for an instance (Repair).
#[tauri::command]
pub async fn repair_instance(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    let dir = paths.game.instance_dir(&folder).map_err(failure)?;
    let instance = load_instance(&dir).map_err(failure)?;
    install_one(
        &app,
        &paths.game,
        &paths.settings,
        &instance.version_id,
        &instance.version_id,
    )
    .await
    .map_err(failure)?;
    install_modpack_files(app, paths, folder).await
}

async fn install_one(
    app: &AppHandle,
    game: &SharedCache,
    settings: &SharedCache,
    version_id: &str,
    progress_version_id: &str,
) -> Result<String, CoreError> {
    emit_install_progress(
        app,
        progress_version_id,
        DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        },
    );

    let cache = game.clone();
    let settings = settings.clone();
    let client = http_client()?;
    let manifest = fetch_version_manifest(&client, &cache).await;
    let entry = match &manifest {
        Ok(manifest) => manifest
            .versions
            .iter()
            .find(|version| version.id == version_id)
            .cloned(),
        Err(_) => None,
    };

    // Custom loader profiles, or offline when only a local JSON exists.
    if entry.is_none() {
        if !local_version_exists(&cache, version_id) {
            return Err(match manifest {
                Err(error) => error,
                Ok(_) => CoreError::VersionMissing,
            });
        }
        if let Some(parent) = local_inherits_from(&cache, version_id)? {
            Box::pin(install_one(
                app,
                game,
                &settings,
                &parent,
                progress_version_id,
            ))
            .await?;
        }
        let resolved = load_resolved_version_json(&cache, version_id)?;
        let version = parse_version(&resolved)?;
        let environment = host_environment()?;
        let (progress, mut incoming) = mpsc::channel(4096);
        let id = version_id.to_string();
        let cache_task = cache.clone();
        let settings_task = settings.clone();
        let installing = tauri::async_runtime::spawn(async move {
            let index_json = client
                .get(&version.asset_index.url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let assets = licht_core::parse_asset_index(&index_json)?;
            licht_core::install_version(
                &client,
                &cache_task,
                &settings_task,
                licht_core::InstallPlan {
                    version_id: &id,
                    version: &version,
                    asset_index_json: &index_json,
                    assets: &assets,
                    environment: &environment,
                    asset_base: ASSET_OBJECT_BASE,
                },
                progress.clone(),
            )
            .await?;
            let major = required_java_major(&id, &version);
            install_temurin(
                &client,
                &cache_task,
                major,
                &environment,
                ADOPTIUM_API,
                progress,
            )
            .await
        });
        let mut throttle = ProgressThrottle::new();
        while let Some(progress) = incoming.recv().await {
            if throttle.should_emit(progress, std::time::Instant::now()) {
                emit_install_progress(app, progress_version_id, progress);
            }
        }
        let java = installing
            .await
            .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
        return Ok(java.display().to_string());
    }

    let entry = entry.expect("checked above");
    let (progress, mut incoming) = mpsc::channel(4096);
    let version_url = entry.url.clone();
    let version_sha1 = entry.sha1.clone();
    let id = version_id.to_string();
    let environment = host_environment()?;
    let installing = tauri::async_runtime::spawn(async move {
        install_game(
            &client,
            &cache,
            &settings,
            GameInstall {
                version_id: &id,
                version_json_url: &version_url,
                version_sha1: &version_sha1,
                environment: &environment,
                asset_base: ASSET_OBJECT_BASE,
                java: None,
            },
            progress,
        )
        .await
    });

    let mut throttle = ProgressThrottle::new();
    while let Some(progress) = incoming.recv().await {
        if throttle.should_emit(progress, std::time::Instant::now()) {
            emit_install_progress(app, progress_version_id, progress);
        }
    }

    let java = installing
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
    Ok(java.display().to_string())
}
