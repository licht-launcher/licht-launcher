use licht_core::{
    DownloadProgress, InstanceEntry, ModpackListItem, ModpackSource, ModpackVersionItem,
    install_curseforge_files, install_mrpack_files, load_instance,
};
use tauri::{AppHandle, State};
use tokio::sync::mpsc;

use crate::progress::ProgressThrottle;

use super::install_progress::emit_install_progress;
use super::{LauncherPaths, failure, http_client};

#[tauri::command]
pub async fn search_modpacks(query: String) -> Result<Vec<ModpackListItem>, String> {
    let client = http_client().map_err(failure)?;
    licht_core::search_modpacks(&client, &query)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn list_modpack_versions(
    source: String,
    project_id: String,
) -> Result<Vec<ModpackVersionItem>, String> {
    let source =
        ModpackSource::parse(&source).ok_or_else(|| "unknown modpack source".to_string())?;
    let client = http_client().map_err(failure)?;
    licht_core::list_modpack_versions(&client, source, &project_id)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn create_instance_from_modpack(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    name: String,
    source: String,
    project_id: String,
    version_id: String,
    icon_url: Option<String>,
) -> Result<InstanceEntry, String> {
    let source =
        ModpackSource::parse(&source).ok_or_else(|| "unknown modpack source".to_string())?;
    downloads
        .inner()
        .create(
            &name,
            licht_core::InstanceSource::Modpack {
                source,
                project_id,
                version_id,
                icon_url,
            },
        )
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn create_instance_from_modpack_file(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    name: String,
    path: String,
) -> Result<InstanceEntry, String> {
    downloads
        .inner()
        .create_from_modpack_file(&name, std::path::Path::new(&path))
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn install_modpack_files(
    app: AppHandle,
    database: State<'_, licht_core::DatabaseClient>,
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    let cache = paths.game.clone();
    let settings = paths.settings.clone();
    let dir = cache.instance_dir(&folder).map_err(failure)?;
    let profile = load_instance(&database, &dir).await.map_err(failure)?;
    let progress_version_id = profile
        .version_id
        .clone()
        .ok_or_else(|| failure(licht_core::CoreError::InstancePreparing))?;
    emit_install_progress(
        &app,
        &progress_version_id,
        DownloadProgress {
            finished: 0,
            failed: 0,
            total: 0,
            bytes_done: 0,
            bytes_total: 0,
        },
    );
    let (progress, mut incoming) = mpsc::channel(4096);
    let database = database.inner().clone();
    let installing = tauri::async_runtime::spawn(async move {
        install_mrpack_files(&database, &cache, &settings, &dir, progress.clone()).await?;
        // CurseForge uses its own allowlisted client (forgecdn hosts).
        install_curseforge_files(&cache, &settings, &dir, progress).await
    });
    let mut throttle = ProgressThrottle::new();
    while let Some(progress) = incoming.recv().await {
        if throttle.should_emit(progress, std::time::Instant::now()) {
            emit_install_progress(&app, &progress_version_id, progress);
        }
    }
    installing
        .await
        .map_err(|_| "install task failed".to_string())?
        .map_err(failure)
}
