use anyhow::Context;

mod commands;
mod logging;
mod progress;

pub fn run() -> anyhow::Result<()> {
    licht_core::load_env_files();
    logging::init()?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Licht Launcher"
    );

    let paths = commands::LauncherPaths::load().context("Failed to resolve Licht folders")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(paths)
        .manage(commands::LaunchSessions::new())
        .invoke_handler(tauri::generate_handler![
            commands::instances::list_instances,
            commands::instances::create_instance,
            commands::instances::list_loader_versions,
            commands::instances::create_instance_with_platform,
            commands::modpacks::search_modpacks,
            commands::modpacks::list_modpack_versions,
            commands::modpacks::create_instance_from_modpack,
            commands::modpacks::install_modpack_files,
            commands::instances::rename_instance,
            commands::instances::duplicate_instance,
            commands::instances::delete_instance,
            commands::instances::save_instance,
            commands::instances::set_instance_version,
            commands::instances::open_instance_folder,
            commands::instances::list_instance_entries,
            commands::shell::open_repository,
            commands::instances::get_last_played_folder,
            commands::instances::set_last_played_folder,
            commands::instances::touch_last_opened,
            commands::instances::clear_last_played_folder,
            commands::accounts::list_microsoft_accounts,
            commands::accounts::active_microsoft_account,
            commands::accounts::begin_microsoft_login,
            commands::accounts::set_active_microsoft_account,
            commands::accounts::logout_microsoft_account,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::runtime_java,
            commands::settings::java_installation_status,
            commands::settings::detect_java_installations,
            commands::settings::install_recommended_java,
            commands::settings::browse_java,
            commands::settings::browse_application_directory,
            commands::versions::list_versions,
            commands::versions::is_version_ready_cmd,
            commands::versions::install_version,
            commands::versions::repair_instance,
            commands::launch::launch,
            commands::launch::list_running_games,
            commands::launch::stop_game
        ])
        .run(tauri::generate_context!())
        .context("Failed to run Licht Launcher")
}
