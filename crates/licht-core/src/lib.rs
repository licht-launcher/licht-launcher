//! Minecraft launcher core, independent of Tauri.
//!
//! Internal layout groups modules by domain (`meta`, `transfer`, `game`,
//! `instance`, `runtime`, `store`). The crate root still exposes the same public
//! names as before so callers and `crate::…` paths inside the library stay stable.

mod auth;
mod error;
mod game;
mod instance;
mod loaders;
mod meta;
mod modpacks;
mod runtime;
mod store;
mod transfer;

pub(crate) use game::install;
pub(crate) use game::launch;
pub(crate) use game::launch_lock;
pub(crate) use game::natives;
pub(crate) use game::offline;
pub(crate) use game::run;
pub(crate) use instance::listing as instance_fs;
pub(crate) use meta::asset_index;
pub(crate) use meta::version;
pub(crate) use meta::version_manifest;
pub(crate) use meta::version_resolve;
pub(crate) use store::cache;
pub(crate) use store::db;
pub(crate) use store::settings;
pub(crate) use transfer::download;

pub use asset_index::{
    AssetIndexFile, AssetObject, fetch_asset_index, parse_asset_index, reconstruct_assets,
};
pub use auth::{
    AccountProfile, EMBEDDED_MSA_CLIENT_ID, MSA_CLIENT_ID_ENV, MicrosoftSession,
    get_active_profile, list_accounts, load_env_files, login_with_browser, logout_account,
    msa_client_id, prepare_microsoft_launch, session_for_launch, set_active_account,
};
pub use cache::SharedCache;
pub use db::{
    clear_last_played_folder, clear_meta, db_path, get_meta, last_played_folder, open_db,
    set_last_played_folder, set_meta,
};
pub use download::{
    DEFAULT_CONCURRENCY, DEFAULT_RETRY, DownloadProgress, DownloadTask, InstallProgress, Retry,
    TransferLimits, download_all, download_file, shared_transfer_limits,
};
pub use error::CoreError;
pub use install::{
    ASSET_OBJECT_BASE, GameInstall, InstallArgs, InstallPlan, install_game, install_version,
    parse_install_args,
};
pub use instance::{
    Instance, InstanceEntry, InstanceSettings, create_instance, delete_instance,
    duplicate_instance, instance_launch, list_instances, load_instance, parse_instance,
    rename_instance, resolved_java_path, resolved_launch, touch_last_opened, update_instance,
    update_instance_version,
};
pub use instance_fs::{InstanceFsEntry, list_instance_entries};
pub use launch::{apply_global_launch, classpath, launch_command};
pub use launch_lock::{
    LaunchLock, clear_launch_lock, kill_launch_process, launch_lock_path,
    list_running_launch_folders, process_matches_lock, process_start_time, read_launch_lock,
    write_launch_lock, write_launch_lock_for_pid,
};
pub use loaders::{
    LoaderChannel, LoaderPlatform, create_instance_with_platform, list_loader_versions,
};
pub use modpacks::{
    CURSEFORGE_API_KEY_ENV, MODPACK_INDEX_FILE, ModpackListItem, ModpackSource, ModpackVersionItem,
    apply_mrpack_loader, create_instance_from_modpack, install_mrpack_files, list_modpack_versions,
    mrpack_download_tasks, search_modpacks,
};
pub use natives::{NativeLibrary, create_natives_directory, extract_natives, native_libraries};
pub use offline::{
    LaunchArgs, OfflineAccount, OfflineLaunch, offline_account, parse_launch_args,
    prepare_offline_launch,
};
pub use run::{GameExit, GameLine, GameLogEvent, LogCodec, OutputStream, log_codec, run_game};
pub use runtime::{
    ADOPTIUM_API, JAVA_RUNTIME_INDEX_URL, JavaChoice, JavaDetections, JavaPaths, JavaRuntimeBuild,
    JavaRuntimeEntry, JavaRuntimeIndex, JavaRuntimeManifest, JavaRuntimeManifestRef, JavaStatus,
    browse_start_directory, default_java_roots, detect_javas, detect_known_javas, discover_javas,
    fetch_java_runtime_index, install_java, install_temurin, installed_java, java_detect_roots,
    java_installation_status, matching_java, parse_java_runtime_index, parse_java_runtime_manifest,
    parse_java_version, probe_java_major, required_java_major, required_runtime,
    runtime_java_paths, runtime_platform, select_runtime, validate_java,
};
pub use settings::{
    GlobalLaunch, Settings, SettingsSnapshot, configured_java, download_concurrency, game_cache,
    global_launch, load_settings, memory_slider_max, recommended_max_memory_mb, save_settings,
    set_java_path, settings_snapshot, total_memory_mb, write_concurrency,
};
pub use version::{
    Arch, Argument, ArgumentValue, Artifact, AssetIndex, Download, GameArguments, JavaVersion,
    LaunchEnvironment, Library, LibraryDownloads, LibraryExtract, OsName, OsRule, Rule, RuleAction,
    Version, VersionDownloads, applicable_libraries, maven_library_path, parse_version,
    rules_allow,
};
pub use version_manifest::{
    LatestVersions, ManifestVersion, VERSION_MANIFEST_URL, VersionManifest, VersionSummary,
    VersionType, VersionsArgs, fetch_version_manifest, parse_version_manifest, parse_versions_args,
    version_lines, version_summaries, version_type_name,
};
pub use version_resolve::{load_resolved_version_json, local_inherits_from, local_version_exists};

pub type Result<T> = std::result::Result<T, CoreError>;
