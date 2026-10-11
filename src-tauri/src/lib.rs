use anyhow::Context;

mod commands;
mod logging;
mod progress;

pub fn run() -> anyhow::Result<()> {
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
            commands::list_instances,
            commands::create_instance,
            commands::rename_instance,
            commands::duplicate_instance,
            commands::delete_instance,
            commands::save_instance,
            commands::set_instance_version,
            commands::open_instance_folder,
            commands::open_repository,
            commands::get_settings,
            commands::save_settings,
            commands::runtime_java,
            commands::java_installation_status,
            commands::detect_java_installations,
            commands::install_recommended_java,
            commands::browse_java,
            commands::browse_application_directory,
            commands::list_versions,
            commands::install_version,
            commands::launch,
            commands::list_running_games,
            commands::stop_game
        ])
        .run(tauri::generate_context!())
        .context("Failed to run Licht Launcher")
}
