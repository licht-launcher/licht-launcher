use licht_core::{
    CoreError, GameExit, GameLogEvent, SharedCache, clear_launch_lock, configured_java,
    global_launch, installed_java, load_instance, load_resolved_version_json, load_settings,
    log_codec, offline_account, parse_version, prepare_microsoft_launch, prepare_offline_launch,
    probe_java_major, required_java_major, resolved_java_path, resolved_launch, run_game,
    session_for_launch, write_launch_lock_for_pid,
};
use std::path::Path;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};
use tracing::Instrument;

use super::sessions::LaunchSessions;
use super::{LauncherPaths, failure, host_environment, http_client};

/// Every event a launch emits carries the instance folder. The player name stays out of logs.
fn launch_span(folder: &str) -> tracing::Span {
    tracing::info_span!("launch", instance = folder)
}

/// Where the Java used for a launch came from.
#[derive(Debug, Clone, Copy)]
enum JavaSource {
    Instance,
    Settings,
    Managed,
}

/// Only the kind of account reaches the log, never the player name.
#[derive(Debug, Clone, Copy)]
enum AccountKind {
    Microsoft,
    Offline,
}

struct LaunchPlan<'a> {
    version: &'a str,
    java: &'a Path,
    java_major: u32,
    java_source: JavaSource,
    max_memory_mb: u32,
    account: AccountKind,
}

fn log_launch_plan(plan: &LaunchPlan<'_>) {
    tracing::info!(
        version = plan.version,
        java = %plan.java.display(),
        java_major = plan.java_major,
        java_source = ?plan.java_source,
        max_memory_mb = plan.max_memory_mb,
        account = ?plan.account,
        "Launching game"
    );
}

/// `None` means the launcher stopped the game.
fn log_game_exit(exit: Option<GameExit>, played: Duration) {
    let played_seconds = played.as_secs();
    match exit.map(|exit| exit.code) {
        Some(Some(0)) => tracing::info!(played_seconds, "Game exited"),
        Some(Some(code)) => tracing::warn!(code, played_seconds, "Game exited with an error"),
        Some(None) => tracing::warn!(played_seconds, "Game ended without an exit code"),
        None => tracing::info!(played_seconds, "Game stopped by the launcher"),
    }
}

#[tauri::command]
pub async fn launch(
    app: AppHandle,
    database: State<'_, licht_core::DatabaseClient>,
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
    let guard = sessions.try_begin(&folder)?;
    let result = launch_one(
        &app,
        &database,
        &paths.game,
        &paths.settings,
        &sessions,
        &folder,
        &username,
    )
    .instrument(launch_span(&folder))
    .await;
    drop(guard);
    if let Err(error) = crate::desktop::game_finished(&app, &folder) {
        tracing::warn!(%error, "Could not apply launcher behavior after the game closed");
    }
    result.map_err(failure)
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
    database: &licht_core::DatabaseClient,
    game: &SharedCache,
    settings_home: &licht_core::Database,
    sessions: &LaunchSessions,
    folder: &str,
    username: &str,
) -> Result<GameExit, CoreError> {
    let cache = game.clone();
    let settings = load_settings(settings_home).await?;
    let dir = cache.instance_dir(folder)?;
    let profile = load_instance(database, &dir).await?;
    let version_id = licht_core::instance_version_for_launch(&profile)?.to_string();
    let json = load_resolved_version_json(&cache, &version_id)?;
    let version = parse_version(&json)?;
    let environment = host_environment()?;
    let required = required_java_major(&version_id, &version);
    let custom_java = resolved_java_path(&profile);
    let (java, java_source) = match &custom_java {
        Some(path) => (path.clone(), JavaSource::Instance),
        None => match configured_java(&settings, required) {
            Some(path) => (path, JavaSource::Settings),
            None => (
                installed_java(&cache, &version_id, &version, &environment)?,
                JavaSource::Managed,
            ),
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
    let session = match session_for_launch(&http, settings_home.cache()).await {
        Ok(session) => Some(session),
        Err(CoreError::AuthNotSignedIn) => None,
        Err(error) => return Err(error),
    };
    log_launch_plan(&LaunchPlan {
        version: &version_id,
        java: &java,
        java_major: major,
        java_source,
        max_memory_mb: launch.max_memory_mb,
        account: if session.is_some() {
            AccountKind::Microsoft
        } else {
            AccountKind::Offline
        },
    });
    let prepared = match session {
        Some(session) => {
            prepare_microsoft_launch(
                database,
                &java,
                &cache,
                &version_id,
                &version,
                &environment,
                &session,
                &dir,
                &launch,
            )
            .await?
        }
        None => {
            let account = offline_account(username)?;
            prepare_offline_launch(
                database,
                &java,
                &cache,
                &version_id,
                &version,
                &environment,
                &account,
                &dir,
                &launch,
            )
            .await?
        }
    };

    let (output, mut incoming) = mpsc::channel(32);
    let (pid_tx, pid_rx) = oneshot::channel();
    let appdata = prepared.appdata.map(|path| path.display().to_string());
    let command = prepared.command;
    let game_directory = prepared.game_directory;
    let started = Instant::now();
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
    if let Ok(pid) = pid_rx.await {
        tracing::info!(pid, "Game started");
        if let Err(error) = write_launch_lock_for_pid(&dir, pid) {
            tracing::warn!(error = %error, folder, "could not write launch lock");
        }
        if let Err(error) = crate::desktop::game_started(app, folder) {
            tracing::warn!(%error, "Could not minimize the launcher after the game started");
        }
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
        Ok(result) => {
            let exit = result?;
            log_game_exit(Some(exit), started.elapsed());
            exit
        }
        Err(join_error) if join_error.is_cancelled() => {
            log_game_exit(None, started.elapsed());
            GameExit { code: None }
        }
        Err(_) => {
            return Err(CoreError::Io(std::io::Error::other("launch task failed")));
        }
    };
    clear_launch_lock(&dir)?;
    Ok(exit)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use licht_core::GameExit;
    use licht_testkit::logs::LogCapture;

    use super::{AccountKind, JavaSource, LaunchPlan, launch_span, log_game_exit, log_launch_plan};

    fn exit_log(exit: Option<GameExit>) -> String {
        let capture = LogCapture::start();
        log_game_exit(exit, Duration::from_secs(754));
        capture.text()
    }

    #[test]
    fn launch_logs_carry_the_instance_folder() {
        let capture = LogCapture::start();
        let _launch = launch_span("Survival").entered();
        tracing::warn!("event inside the launch");
        let logs = capture.text();

        assert!(
            logs.contains("launch{instance=\"Survival\"}"),
            "unexpected logs: {logs}"
        );
        assert!(
            logs.contains("event inside the launch"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn the_launch_plan_names_the_version_java_and_account_kind() {
        let capture = LogCapture::start();
        log_launch_plan(&LaunchPlan {
            version: "fabric-loader-0.16.14-1.21.4",
            java: Path::new("runtime/java-21/bin/java"),
            java_major: 21,
            java_source: JavaSource::Managed,
            max_memory_mb: 4096,
            account: AccountKind::Offline,
        });
        let logs = capture.text();

        assert!(logs.contains("INFO"), "unexpected logs: {logs}");
        assert!(logs.contains("Launching game"), "unexpected logs: {logs}");
        for field in [
            "version=\"fabric-loader-0.16.14-1.21.4\"",
            "java=runtime/java-21/bin/java",
            "java_major=21",
            "java_source=Managed",
            "max_memory_mb=4096",
            "account=Offline",
        ] {
            assert!(logs.contains(field), "missing {field}: {logs}");
        }
    }

    #[test]
    fn a_clean_exit_is_logged_with_the_time_played() {
        let logs = exit_log(Some(GameExit { code: Some(0) }));

        assert!(logs.contains("INFO"), "unexpected logs: {logs}");
        assert!(logs.contains("Game exited"), "unexpected logs: {logs}");
        assert!(
            logs.contains("played_seconds=754"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn a_failed_exit_is_a_warning_with_its_code() {
        let logs = exit_log(Some(GameExit { code: Some(-1) }));

        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(
            logs.contains("Game exited with an error"),
            "unexpected logs: {logs}"
        );
        assert!(logs.contains("code=-1"), "unexpected logs: {logs}");
    }

    #[test]
    fn an_exit_without_a_code_is_a_warning() {
        let logs = exit_log(Some(GameExit { code: None }));

        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(
            logs.contains("Game ended without an exit code"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn a_game_stopped_by_the_launcher_is_not_a_warning() {
        let logs = exit_log(None);

        assert!(logs.contains("INFO"), "unexpected logs: {logs}");
        assert!(
            logs.contains("Game stopped by the launcher"),
            "unexpected logs: {logs}"
        );
    }
}
