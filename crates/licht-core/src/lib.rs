//! Minecraft launcher core, independent of Tauri.
//!
//! Internal layout groups modules by domain (`meta`, `transfer`, `game`,
//! `instance`, `runtime`, `store`). The crate root still exposes the same public
//! names as before so callers and `crate::…` paths inside the library stay stable.

mod auth;
mod diagnostics;
mod error;
mod game;
mod hashes;
mod instance;
mod launcher_behavior;
mod loaders;
mod meta;
mod modpacks;
mod path_component;
mod runtime;
mod store;
#[cfg(test)]
mod test_http;
mod transfer;
mod updates;

pub use updates::{
    APP_UPDATE_INTERVAL_SECONDS, AppUpdateCache, AppUpdatePackage, AppUpdatePhase, AppUpdateState,
    app_update_can_check,
};

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
    AccountProfile, DEFAULT_SKIN_URL, EMBEDDED_MSA_CLIENT_ID, MSA_CLIENT_ID_ENV, MicrosoftSession,
    cached_player_head, get_active_profile, list_accounts, load_env_files, login_with_browser,
    logout_account, msa_client_id, prepare_microsoft_launch, session_for_launch,
    set_active_account, use_offline_account,
};
pub use cache::SharedCache;
pub use db::{
    clear_last_played_folder, clear_meta, db_path, get_meta, last_played_folder, open_db,
    set_last_played_folder, set_meta,
};
pub use diagnostics::{BuildDiagnostics, build_diagnostics};
pub use download::{
    DEFAULT_CONCURRENCY, DEFAULT_RETRY, DownloadOptions, DownloadProgress, DownloadTask,
    InstallProgress, Retry, TransferLimits, VerifiedDownloadTask, download_all,
    download_all_with_control, download_file, download_file_with_control, download_verified_all,
    shared_transfer_limits,
};
pub use error::CoreError;
pub use install::{
    ASSET_OBJECT_BASE, GameInstall, InstallArgs, InstallPlan, install_game, install_game_version,
    install_version, parse_install_args,
};
pub use instance::content::{
    InstanceContentEntry, InstanceContentKind, InstanceContentUpdate, list_instance_content,
    list_instance_content_progressively, list_instance_content_with_metadata,
    remove_instance_content, set_instance_content_enabled,
};
pub use instance::icon::{ICON_FILE, download_instance_icon, icon_url_allowed};
pub use instance::{
    Instance, InstanceEntry, InstanceInstallation, InstancePreparationState, InstanceSettings,
    InstanceSource, complete_instance_preparation, configure_instance_version, create_instance,
    create_pending_instance, delete_instance, duplicate_instance, instance_entry, instance_launch,
    instance_version_for_launch, list_instances, load_instance, parse_instance, rename_instance,
    resolved_java_path, resolved_launch, touch_last_opened, update_instance,
    update_instance_version,
};
pub use instance_fs::{InstanceFsEntry, list_instance_entries};
pub use launch::{apply_global_launch, classpath, launch_command};
pub use launch_lock::{
    LaunchLock, clear_launch_lock, kill_launch_process, launch_lock_path,
    list_running_launch_folders, process_matches_lock, process_start_time, read_launch_lock,
    write_launch_lock, write_launch_lock_for_pid,
};
pub use launcher_behavior::LauncherBehavior;
pub use loaders::{
    LoaderChannel, LoaderPlatform, create_instance_with_platform, list_loader_versions,
};
pub use modpacks::{
    CURSEFORGE_API_KEY_ENV, CURSEFORGE_INDEX_FILE, MODPACK_INDEX_FILE, ModpackListItem,
    ModpackSource, ModpackVersionItem, apply_mrpack_loader, create_instance_from_modpack,
    install_curseforge_files, install_mrpack_files, list_modpack_versions, mrpack_download_tasks,
    search_modpacks,
};
pub use natives::{NativeLibrary, create_natives_directory, extract_natives, native_libraries};
pub use offline::{
    LaunchArgs, OfflineAccount, OfflineLaunch, offline_account, parse_launch_args,
    prepare_offline_launch,
};
pub use run::{GameExit, GameLine, GameLogEvent, LogCodec, OutputStream, log_codec, run_game};
pub use runtime::{
    ADOPTIUM_API, JavaDetections, JavaPaths, JavaStatus, browse_start_directory,
    default_java_roots, detect_javas, detect_known_javas, discover_javas, install_temurin,
    installed_java, java_detect_roots, java_installation_status, matching_java, parse_java_version,
    probe_java_major, required_java_major, required_runtime, runtime_java_paths, use_custom_java,
    validate_java,
};
pub use settings::{
    GlobalLaunch, LauncherLanguage, LauncherTheme, Settings, SettingsSnapshot, configured_java,
    download_concurrency, game_cache, global_launch, load_settings, memory_slider_max,
    recommended_max_memory_mb, save_settings, set_java_path, settings_snapshot, total_memory_mb,
    write_concurrency,
};
pub use store::layout::progress::{
    StartupPhase, StartupPreviewScenario, StartupProgress, StartupReporter, StartupUpdateStatus,
};
pub use store::layout::requests::{
    StartupUpdatePolicy, clear_startup_update_request, mark_startup_update_attempt,
    read_startup_update_intent, request_startup_preview, request_startup_update,
    startup_update_policy, take_startup_preview,
};
pub use store::layout::{
    initialize_launcher, initialize_launcher_with_progress, run_startup_migration_preview,
    startup_settings,
};
pub use store::service::{DatabaseClient, DatabaseService};
pub use store::storage::{StorageCategory, StorageSummary, StorageUsage, storage_summary};
pub use transfer::checksum::DownloadChecksum;
pub use transfer::control::{DownloadControl, DownloadState, DownloadTelemetry};
pub use transfer::installation::InstallationContext;
pub use transfer::jobs::{
    DownloadComponent, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus,
    DownloadJobUpdate, DownloadPhase, create_download_job, create_pending_download_job,
    get_download_job, list_download_jobs, recover_download_jobs, update_download_job,
};
pub use transfer::manager::DownloadManager;
pub use version::{
    Arch, Argument, ArgumentValue, Artifact, AssetIndex, Download, GameArguments, JavaVersion,
    LaunchEnvironment, Library, LibraryDownloads, LibraryExtract, OsName, OsRule, Rule, RuleAction,
    Version, VersionDownloads, applicable_libraries, maven_library_path, parse_version,
    rules_allow,
};
pub use version_manifest::{
    LatestVersions, MANIFEST_REQUEST_TIMEOUT, ManifestVersion, VERSION_MANIFEST_LEGACY_URL,
    VERSION_MANIFEST_URL, VersionManifest, VersionSummary, VersionType, VersionsArgs,
    fetch_version_manifest, is_version_ready, list_installed_version_summaries,
    list_version_summaries, parse_version_manifest, parse_versions_args, version_lines,
    version_manifest_cache_path, version_summaries, version_type_name,
};
pub use version_resolve::{load_resolved_version_json, local_inherits_from, local_version_exists};

pub type Result<T> = std::result::Result<T, CoreError>;
