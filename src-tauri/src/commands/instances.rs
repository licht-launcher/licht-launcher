use licht_core::{
    DatabaseClient, InstanceContentEntry, InstanceContentKind, InstanceEntry, InstanceFsEntry,
    InstanceSettings, LoaderChannel, LoaderPlatform, remove_instance_content as remove_content,
    set_instance_content_enabled as set_content_enabled,
    touch_last_opened as touch_instance_opened, update_instance,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use super::{LauncherPaths, failure, http_client, instances_root};

#[tauri::command]
pub fn list_instances(paths: State<'_, LauncherPaths>) -> Result<Vec<InstanceEntry>, String> {
    let root = instances_root(&paths).map_err(failure)?;
    licht_core::list_instances(&root).map_err(failure)
}

#[tauri::command]
pub async fn create_instance(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    name: String,
    version_id: String,
) -> Result<InstanceEntry, String> {
    downloads
        .inner()
        .create(
            &name,
            licht_core::InstanceSource::Vanilla {
                game_version: version_id,
            },
        )
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn list_loader_versions(
    platform: String,
    game_version: String,
    channel: String,
) -> Result<Vec<String>, String> {
    let platform =
        LoaderPlatform::parse(&platform).ok_or_else(|| "unknown platform".to_string())?;
    let channel = LoaderChannel::parse(&channel).ok_or_else(|| "unknown channel".to_string())?;
    let client = http_client().map_err(failure)?;
    licht_core::list_loader_versions(&client, platform, &game_version, channel)
        .await
        .map_err(failure)
}

#[tauri::command]
pub async fn create_instance_with_platform(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    name: String,
    platform: String,
    game_version: String,
    loader_channel: String,
    loader_version: Option<String>,
) -> Result<InstanceEntry, String> {
    let platform =
        LoaderPlatform::parse(&platform).ok_or_else(|| "unknown platform".to_string())?;
    let channel =
        LoaderChannel::parse(&loader_channel).ok_or_else(|| "unknown channel".to_string())?;
    let source = if platform == LoaderPlatform::Vanilla {
        licht_core::InstanceSource::Vanilla { game_version }
    } else {
        licht_core::InstanceSource::Loader {
            platform,
            game_version,
            channel,
            loader_version,
        }
    };
    downloads
        .inner()
        .create(&name, source)
        .await
        .map_err(failure)
}

#[tauri::command]
pub fn rename_instance(
    paths: State<'_, LauncherPaths>,
    folder: String,
    name: String,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    licht_core::rename_instance(&root, &folder, &name).map_err(failure)
}

#[tauri::command]
pub fn duplicate_instance(
    paths: State<'_, LauncherPaths>,
    folder: String,
    name: String,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    licht_core::duplicate_instance(&root, &folder, &name).map_err(failure)
}

#[tauri::command]
pub async fn delete_instance(
    paths: State<'_, LauncherPaths>,
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    folder: String,
) -> Result<(), String> {
    downloads.cancel_instance(&folder).await.map_err(failure)?;
    let root = instances_root(&paths).map_err(failure)?;
    licht_core::delete_instance(&root, &folder).map_err(failure)
}

#[tauri::command]
pub fn save_instance(
    paths: State<'_, LauncherPaths>,
    folder: String,
    settings: InstanceSettings,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    update_instance(&root, &folder, &settings).map_err(failure)
}

#[tauri::command]
pub async fn set_instance_version(
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    folder: String,
    version_id: String,
) -> Result<InstanceEntry, String> {
    downloads
        .change_version(&folder, &version_id)
        .await
        .map_err(failure)
}

#[tauri::command]
pub fn open_instance_folder(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    let path = paths.game.instance_dir(&folder).map_err(failure)?;
    if !path.is_dir() {
        return Err("instance directory is missing".to_string());
    }
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_instance_entries(
    paths: State<'_, LauncherPaths>,
    folder: String,
    relative_path: String,
) -> Result<Vec<InstanceFsEntry>, String> {
    licht_core::list_instance_entries(&paths.game, &folder, &relative_path).map_err(failure)
}

#[tauri::command]
pub async fn list_instance_content(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    database: State<'_, DatabaseClient>,
    folder: String,
    refresh: bool,
    request_id: String,
) -> Result<Vec<InstanceContentEntry>, String> {
    if request_id.is_empty() || request_id.len() > 128 {
        return Err("invalid content request identifier".to_string());
    }
    let client = http_client().map_err(failure)?;
    licht_core::list_instance_content_progressively(
        &paths.game,
        &database,
        &client,
        &folder,
        refresh,
        |entries| {
            app.emit(
                "instance-content-updated",
                licht_core::InstanceContentUpdate {
                    request_id: request_id.clone(),
                    instance_folder: folder.clone(),
                    entries: entries.to_vec(),
                },
            )
            .map_err(std::io::Error::other)?;
            Ok(())
        },
    )
    .await
    .map_err(failure)
}

#[tauri::command]
pub fn set_instance_content_enabled(
    paths: State<'_, LauncherPaths>,
    folder: String,
    kind: InstanceContentKind,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    set_content_enabled(&paths.game, &folder, kind, &name, enabled).map_err(failure)
}

#[tauri::command]
pub fn remove_instance_content(
    paths: State<'_, LauncherPaths>,
    folder: String,
    kind: InstanceContentKind,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    remove_content(&paths.game, &folder, kind, &name, enabled).map_err(failure)
}

#[tauri::command]
pub async fn get_last_played_folder(
    database: State<'_, DatabaseClient>,
) -> Result<Option<String>, String> {
    database.last_played_folder().await.map_err(failure)
}

#[tauri::command]
pub async fn set_last_played_folder(
    database: State<'_, DatabaseClient>,
    folder: String,
) -> Result<(), String> {
    database
        .set_last_played_folder(&folder)
        .await
        .map_err(failure)
}

/// Marks the instance as opened now and updates the global last-played folder.
#[tauri::command]
pub async fn touch_last_opened(
    paths: State<'_, LauncherPaths>,
    database: State<'_, DatabaseClient>,
    folder: String,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    let profile_folder = folder.clone();
    let entry =
        tauri::async_runtime::spawn_blocking(move || touch_instance_opened(&root, &profile_folder))
            .await
            .map_err(|_| "instance timestamp task failed".to_string())?
            .map_err(failure)?;
    database
        .set_last_played_folder(&folder)
        .await
        .map_err(failure)?;
    Ok(entry)
}

#[tauri::command]
pub async fn clear_last_played_folder(database: State<'_, DatabaseClient>) -> Result<(), String> {
    database.clear_last_played_folder().await.map_err(failure)
}
