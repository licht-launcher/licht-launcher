use licht_core::{
    CoreError, GameExit, GameLogEvent, SharedCache, clear_launch_lock, configured_java,
    global_launch, installed_java, load_instance, load_resolved_version_json, load_settings,
    log_codec, offline_account, parse_version, prepare_microsoft_launch, prepare_offline_launch,
    probe_java_major, required_java_major, resolved_java_path, resolved_launch, run_game,
    session_for_launch, write_launch_lock_for_pid,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

use super::sessions::LaunchSessions;
use super::{LauncherPaths, failure, host_environment, http_client};

#[tauri::command]
pub async fn launch(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    sessions: State<'_, LaunchSessions>,
    downloads: State<'_, std::sync::Arc<licht_core::DownloadManager>>,
    folder: String,
    username: String,
) -> Result<GameExit, String> {
    downloads
        .ensure_launchable(&folder)
        .await
        .map_err(failure)?;
    let instances_dir = paths.game.instances_dir().map_err(failure)?;
    sessions.reconcile(&instances_dir)?;
    let _guard = sessions.try_begin(&folder)?;
    launch_one(
        &app,
        &paths.game,
        &paths.settings,
        &sessions,
        &folder,
        &username,
    )
    .await
    .map_err(failure)
}

#[tauri::command]
pub fn list_running_games(
    paths: State<'_, LauncherPaths>,
    sessions: State<'_, LaunchSessions>,
) -> Result<Vec<String>, String> {
    let instances_dir = paths.game.instances_dir().map_err(failure)?;
    sessions.list_running(&instances_dir)
}

#[tauri::command]
pub fn stop_game(
    paths: State<'_, LauncherPaths>,
    sessions: State<'_, LaunchSessions>,
    folder: String,
) -> Result<(), String> {
    let instances_dir = paths.game.instances_dir().map_err(failure)?;
    sessions.reconcile(&instances_dir)?;
    let dir = paths.game.instance_dir(&folder).map_err(failure)?;
    sessions.stop(&folder, &dir)
}

async fn launch_one(
    app: &AppHandle,
    game: &SharedCache,
    settings_home: &SharedCache,
    sessions: &LaunchSessions,
    folder: &str,
    username: &str,
) -> Result<GameExit, CoreError> {
    let cache = game.clone();
    let settings = load_settings(settings_home)?;
    let dir = cache.instance_dir(folder)?;
    let profile = load_instance(&dir)?;
    let version_id = licht_core::instance_version_for_launch(&profile)?.to_string();
    let json = load_resolved_version_json(&cache, &version_id)?;
    let version = parse_version(&json)?;
    let environment = host_environment()?;
    let required = required_java_major(&version_id, &version);
    let custom_java = resolved_java_path(&profile);
    let java = match &custom_java {
        Some(path) => path.clone(),
        None => match configured_java(&settings, required) {
            Some(path) => path,
            None => installed_java(&cache, &version_id, &version, &environment)?,
        },
    };
    let major = if custom_java.is_some() {
        probe_java_major(&java)?
    } else {
        required
    };
    let codec = log_codec(major);
    let launch = resolved_launch(&profile, &global_launch(&settings));
    let http = http_client()?;
    let prepared = match session_for_launch(&http, settings_home).await {
        Ok(session) => prepare_microsoft_launch(
            &java,
            &cache,
            &version_id,
            &version,
            &environment,
            &session,
            &dir,
            &launch,
        )?,
        Err(CoreError::AuthNotSignedIn) => {
            let account = offline_account(username)?;
            prepare_offline_launch(
                &java,
                &cache,
                &version_id,
                &version,
                &environment,
                &account,
                &dir,
                &launch,
            )?
        }
        Err(error) => return Err(error),
    };

    let (output, mut incoming) = mpsc::channel(32);
    let (pid_tx, pid_rx) = oneshot::channel();
    let appdata = prepared.appdata.map(|path| path.display().to_string());
    let command = prepared.command;
    let game_directory = prepared.game_directory;
    // tokio::spawn exposes AbortHandle; tauri::async_runtime::JoinHandle does not.
    let running = tokio::spawn(async move {
        let mut env = Vec::new();
        if let Some(path) = &appdata {
            env.push(("APPDATA", path.as_str()));
        }
        run_game(
            &command,
            Some(&game_directory),
            &env,
            codec,
            output,
            Some(pid_tx),
        )
        .await
    });
    sessions
        .attach_live(folder, running.abort_handle())
        .map_err(|message| CoreError::Io(std::io::Error::other(message)))?;
    if let Ok(pid) = pid_rx.await
        && let Err(error) = write_launch_lock_for_pid(&dir, pid)
    {
        tracing::warn!(error = %error, folder, "could not write launch lock");
    }

    while let Some(line) = incoming.recv().await {
        let _ = app.emit(
            "game-log",
            GameLogEvent {
                folder: folder.to_string(),
                stream: line.stream,
                line: line.line,
            },
        );
    }
    let exit = match running.await {
        Ok(result) => result?,
        Err(join_error) if join_error.is_cancelled() => GameExit { code: None },
        Err(_) => {
            return Err(CoreError::Io(std::io::Error::other("launch task failed")));
        }
    };
    clear_launch_lock(&dir)?;
    Ok(exit)
}
