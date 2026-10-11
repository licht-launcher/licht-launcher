use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex as StdMutex;

use licht_core::{
    ADOPTIUM_API, ASSET_OBJECT_BASE, AccountProfile, Arch, CoreError, DeviceCodeChallenge,
    GameExit, GameInstall, GameLogEvent, InstallProgress, InstanceEntry, InstanceFsEntry,
    InstanceSettings, JavaDetections, JavaPaths, JavaStatus, LaunchEnvironment, OsName, Settings,
    SettingsSnapshot, SharedCache, VersionSummary, begin_login, browse_start_directory,
    clear_launch_lock, configured_java, detect_known_javas, fetch_version_manifest, game_cache,
    get_active_profile, global_launch, install_game, install_temurin, installed_java,
    kill_launch_process, list_accounts, list_running_launch_folders, load_instance, load_settings,
    log_codec, logout_account, offline_account, parse_version, poll_login,
    prepare_microsoft_launch, prepare_offline_launch, probe_java_major, process_matches_lock,
    read_launch_lock, required_java_major, resolved_java_path, resolved_launch, run_game,
    runtime_java_paths, session_for_launch, set_active_account, set_java_path, settings_snapshot,
    update_instance, update_instance_version, version_summaries, write_launch_lock_for_pid,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{mpsc, oneshot};
use tokio::task::AbortHandle;

use crate::progress::ProgressThrottle;

enum LaunchSession {
    Starting,
    Live { abort: AbortHandle },
    Orphan { pid: u32 },
}

/// Tracks which instance folders currently have a running game.
pub struct LaunchSessions {
    active: StdMutex<HashMap<String, LaunchSession>>,
}

/// Removes the folder from [`LaunchSessions`] when dropped.
pub struct LaunchGuard<'a> {
    sessions: &'a LaunchSessions,
    folder: String,
}

impl LaunchSessions {
    pub fn new() -> Self {
        Self {
            active: StdMutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, LaunchSession>>, String> {
        self.active
            .lock()
            .map_err(|_| "launch session lock is poisoned".to_string())
    }

    pub fn try_begin(&self, folder: &str) -> Result<LaunchGuard<'_>, String> {
        let mut active = self.lock()?;
        if active.contains_key(folder) {
            return Err(format!("instance \"{folder}\" is already running"));
        }
        active.insert(folder.to_string(), LaunchSession::Starting);
        Ok(LaunchGuard {
            sessions: self,
            folder: folder.to_string(),
        })
    }

    fn attach_live(&self, folder: &str, abort: AbortHandle) -> Result<(), String> {
        let mut active = self.lock()?;
        match active.get_mut(folder) {
            Some(slot @ LaunchSession::Starting) => {
                *slot = LaunchSession::Live { abort };
                Ok(())
            }
            Some(_) => Err(format!("instance \"{folder}\" is already running")),
            None => Err(format!("instance \"{folder}\" has no launch session")),
        }
    }

    fn reconcile(&self, instances_dir: &Path) -> Result<(), String> {
        let found = list_running_launch_folders(instances_dir).map_err(failure)?;
        let mut active = self.lock()?;
        for folder in found {
            if active.contains_key(&folder) {
                continue;
            }
            let dir = instances_dir.join(&folder);
            let Some(lock) = read_launch_lock(&dir).map_err(failure)? else {
                continue;
            };
            active.insert(folder, LaunchSession::Orphan { pid: lock.pid });
        }

        let stale: Vec<String> = active
            .iter()
            .filter_map(|(folder, session)| match session {
                LaunchSession::Orphan { .. } => {
                    let dir = instances_dir.join(folder);
                    match read_launch_lock(&dir) {
                        Ok(Some(lock)) if process_matches_lock(lock.pid, lock.start_time) => None,
                        _ => Some(folder.clone()),
                    }
                }
                _ => None,
            })
            .collect();
        for folder in stale {
            active.remove(&folder);
            let _ = clear_launch_lock(&instances_dir.join(&folder));
        }
        Ok(())
    }

    fn list_running(&self, instances_dir: &Path) -> Result<Vec<String>, String> {
        self.reconcile(instances_dir)?;
        let active = self.lock()?;
        let mut folders: Vec<String> = active.keys().cloned().collect();
        folders.sort();
        Ok(folders)
    }

    fn stop(&self, folder: &str, instance_dir: &Path) -> Result<(), String> {
        enum Action {
            Abort(AbortHandle),
            Kill(u32),
            Busy,
            Missing,
        }

        let action = {
            let mut active = self.lock()?;
            match active.get(folder) {
                Some(LaunchSession::Live { abort }) => Action::Abort(abort.clone()),
                Some(LaunchSession::Orphan { pid }) => {
                    let pid = *pid;
                    active.remove(folder);
                    Action::Kill(pid)
                }
                Some(LaunchSession::Starting) => Action::Busy,
                None => Action::Missing,
            }
        };

        match action {
            Action::Abort(abort) => {
                abort.abort();
                Ok(())
            }
            Action::Kill(pid) => {
                kill_launch_process(pid).map_err(failure)?;
                clear_launch_lock(instance_dir).map_err(failure)?;
                Ok(())
            }
            Action::Busy => Err(format!("instance \"{folder}\" is still starting")),
            Action::Missing => {
                if let Some(lock) = read_launch_lock(instance_dir).map_err(failure)? {
                    if process_matches_lock(lock.pid, lock.start_time) {
                        kill_launch_process(lock.pid).map_err(failure)?;
                    }
                    clear_launch_lock(instance_dir).map_err(failure)?;
                }
                Ok(())
            }
        }
    }
}

impl Drop for LaunchGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.sessions.active.lock() {
            active.remove(&self.folder);
        }
    }
}

/// Folders chosen when the process started. A saved data directory is used on the
/// next start, so this process keeps the folders it opened with.
#[derive(Clone)]
pub struct LauncherPaths {
    pub settings: SharedCache,
    pub game: SharedCache,
}

impl LauncherPaths {
    pub fn load() -> Result<Self, CoreError> {
        let settings = SharedCache::system()?;
        let game = game_cache(&settings)?;
        Ok(Self { settings, game })
    }
}

fn failure(error: CoreError) -> String {
    error.to_string()
}

fn host_environment() -> Result<LaunchEnvironment, CoreError> {
    let os = match std::env::consts::OS {
        "windows" => OsName::Windows,
        "linux" => OsName::Linux,
        "macos" => OsName::Osx,
        _ => return Err(CoreError::LaunchHost),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "x86" => Arch::X86,
        _ => return Err(CoreError::LaunchHost),
    };
    Ok(LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    })
}

fn instances_root(paths: &LauncherPaths) -> Result<std::path::PathBuf, CoreError> {
    paths.game.instances_dir()
}

#[tauri::command]
pub fn list_instances(paths: State<'_, LauncherPaths>) -> Result<Vec<InstanceEntry>, String> {
    let root = instances_root(&paths).map_err(failure)?;
    licht_core::list_instances(&root).map_err(failure)
}

#[tauri::command]
pub fn create_instance(
    paths: State<'_, LauncherPaths>,
    name: String,
    version_id: String,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    let settings = load_settings(&paths.settings).map_err(failure)?;
    licht_core::create_instance(&root, &name, &version_id, &global_launch(&settings))
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
pub fn delete_instance(paths: State<'_, LauncherPaths>, folder: String) -> Result<(), String> {
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
pub fn set_instance_version(
    paths: State<'_, LauncherPaths>,
    folder: String,
    version_id: String,
) -> Result<InstanceEntry, String> {
    let root = instances_root(&paths).map_err(failure)?;
    update_instance_version(&root, &folder, &version_id).map_err(failure)
}

const REPOSITORY_URL: &str = "https://github.com/mateusneiva/licht-launcher";

#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|error| error.to_string())
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
pub fn get_last_played_folder(paths: State<'_, LauncherPaths>) -> Result<Option<String>, String> {
    licht_core::last_played_folder(&paths.settings).map_err(failure)
}

#[tauri::command]
pub fn set_last_played_folder(
    paths: State<'_, LauncherPaths>,
    folder: String,
) -> Result<(), String> {
    licht_core::set_last_played_folder(&paths.settings, &folder).map_err(failure)
}

#[tauri::command]
pub fn clear_last_played_folder(paths: State<'_, LauncherPaths>) -> Result<(), String> {
    licht_core::clear_last_played_folder(&paths.settings).map_err(failure)
}

#[tauri::command]
pub fn list_microsoft_accounts(
    paths: State<'_, LauncherPaths>,
) -> Result<Vec<AccountProfile>, String> {
    list_accounts(&paths.settings).map_err(failure)
}

#[tauri::command]
pub fn active_microsoft_account(
    paths: State<'_, LauncherPaths>,
) -> Result<Option<AccountProfile>, String> {
    get_active_profile(&paths.settings).map_err(failure)
}

#[tauri::command]
pub async fn begin_microsoft_login() -> Result<DeviceCodeChallenge, String> {
    let client = reqwest::Client::new();
    begin_login(&client).await.map_err(failure)
}

#[tauri::command]
pub async fn poll_microsoft_login(
    paths: State<'_, LauncherPaths>,
    device_code: String,
) -> Result<Option<AccountProfile>, String> {
    let client = reqwest::Client::new();
    poll_login(&client, &paths.settings, &device_code)
        .await
        .map_err(failure)
}

#[tauri::command]
pub fn set_active_microsoft_account(
    paths: State<'_, LauncherPaths>,
    uuid: String,
) -> Result<AccountProfile, String> {
    set_active_account(&paths.settings, &uuid).map_err(failure)
}

#[tauri::command]
pub fn logout_microsoft_account(
    paths: State<'_, LauncherPaths>,
    uuid: String,
) -> Result<(), String> {
    logout_account(&paths.settings, &uuid).map_err(failure)
}

#[tauri::command]
pub fn get_settings(paths: State<'_, LauncherPaths>) -> Result<SettingsSnapshot, String> {
    settings_snapshot(&paths.settings).map_err(failure)
}

#[tauri::command]
pub fn save_settings(
    paths: State<'_, LauncherPaths>,
    settings: Settings,
) -> Result<Settings, String> {
    licht_core::save_settings(&paths.settings, &settings).map_err(failure)
}

#[tauri::command]
pub fn runtime_java(paths: State<'_, LauncherPaths>) -> Result<JavaPaths, String> {
    let environment = host_environment().map_err(failure)?;
    runtime_java_paths(&paths.game, &environment).map_err(failure)
}

#[tauri::command]
pub async fn java_installation_status(paths: JavaPaths) -> Result<JavaStatus, String> {
    tauri::async_runtime::spawn_blocking(move || licht_core::java_installation_status(&paths))
        .await
        .map_err(|_| "Java check failed".to_string())
}

#[tauri::command]
pub async fn detect_java_installations(
    paths: State<'_, LauncherPaths>,
) -> Result<JavaDetections, String> {
    let game = paths.game.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let environment = host_environment().map_err(failure)?;
        Ok(detect_known_javas(&game, environment.os))
    })
    .await
    .map_err(|_| "Java check failed".to_string())?
}

#[tauri::command]
pub async fn install_recommended_java(
    paths: State<'_, LauncherPaths>,
    major: u32,
) -> Result<String, String> {
    if !matches!(major, 25 | 21 | 17 | 8) {
        return Err(CoreError::SettingsJava.to_string());
    }
    let game = paths.game.clone();
    let settings = paths.settings.clone();
    let environment = host_environment().map_err(failure)?;
    let executable = install_temurin(
        &reqwest::Client::new(),
        &game,
        major,
        &environment,
        ADOPTIUM_API,
        tokio::sync::mpsc::channel(8).0,
    )
    .await
    .map_err(failure)?;
    set_java_path(&settings, major, &executable).map_err(failure)?;
    Ok(executable.display().to_string())
}

fn picked_path(file: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<String>, String> {
    let Some(file) = file else {
        return Ok(None);
    };
    match file.into_path() {
        Ok(path) => Ok(Some(path.display().to_string())),
        Err(error) => Err(error.to_string()),
    }
}

fn starting_directory(current: &str, fallback: &Path) -> Result<PathBuf, String> {
    let directory = browse_start_directory(current, fallback);
    if !directory.is_dir() {
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    }
    Ok(directory)
}

fn owned_file_dialog(app: &AppHandle) -> FileDialogBuilder<tauri::Wry> {
    let mut dialog = app.dialog().file();
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    dialog
}

#[tauri::command]
pub async fn browse_java(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let directory = starting_directory(&path, &paths.game.root().join("runtime"))?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&app)
        .set_directory(&directory)
        .pick_file(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver.await.map_err(|_| "browse failed".to_string())?;
    picked_path(file)
}

#[tauri::command]
pub async fn browse_application_directory(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    path: String,
) -> Result<Option<String>, String> {
    let Some(fallback) = paths.settings.root().parent() else {
        return Err("application directory is missing".to_string());
    };
    let directory = starting_directory(&path, fallback)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    owned_file_dialog(&app)
        .set_directory(&directory)
        .pick_folder(move |file| {
            let _ = sender.send(file);
        });
    let file = receiver.await.map_err(|_| "browse failed".to_string())?;
    picked_path(file)
}

#[tauri::command]
pub async fn list_versions(paths: State<'_, LauncherPaths>) -> Result<Vec<VersionSummary>, String> {
    let cache = paths.game.clone();
    let manifest = fetch_version_manifest(&reqwest::Client::new())
        .await
        .map_err(failure)?;
    Ok(version_summaries(&manifest, &cache))
}

#[tauri::command]
pub async fn install_version(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    version_id: String,
) -> Result<String, String> {
    install_one(&app, &paths.game, &paths.settings, &version_id)
        .await
        .map_err(failure)
}

async fn install_one(
    app: &AppHandle,
    game: &SharedCache,
    settings: &SharedCache,
    version_id: &str,
) -> Result<String, CoreError> {
    let cache = game.clone();
    let settings = settings.clone();
    let client = reqwest::Client::new();
    let manifest = fetch_version_manifest(&client).await?;
    let Some(entry) = manifest
        .versions
        .iter()
        .find(|version| version.id == version_id)
    else {
        return Err(CoreError::VersionMissing);
    };

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
            let _ = app.emit(
                "install-progress",
                InstallProgress::tagged(version_id, progress),
            );
        }
    }

    let java = installing
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
    Ok(java.display().to_string())
}

#[tauri::command]
pub async fn launch(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
    sessions: State<'_, LaunchSessions>,
    folder: String,
    username: String,
) -> Result<GameExit, String> {
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
    let version_id = profile.version_id.clone();
    let json = std::fs::read_to_string(cache.version_json(&version_id)?)?;
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
    let http = reqwest::Client::new();
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
