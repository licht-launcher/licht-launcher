use anyhow::Context;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

mod commands;
mod logging;
mod progress;

/// Logical height of the custom title bar strip (matches frontend `h-9`).
#[cfg(windows)]
const TITLE_BAR_HEIGHT: u32 = 36;

pub fn run() -> anyhow::Result<()> {
    licht_core::load_env_files();
    logging::init()?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Licht Launcher"
    );

    let paths = tauri::async_runtime::block_on(commands::LauncherPaths::load())
        .context("Failed to resolve Licht folders")?;
    let database =
        tauri::async_runtime::block_on(licht_core::DatabaseService::start(paths.settings.clone()))
            .context("Failed to start the database service")?;
    tauri::async_runtime::block_on(database.client().recover_download_jobs(&paths.game))
        .context("Failed to recover download jobs")?;
    let client = reqwest::Client::builder()
        .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .context("Failed to create the download client")?;
    let downloads = std::sync::Arc::new(licht_core::DownloadManager::new(
        paths.settings.clone(),
        paths.game.clone(),
        commands::host_environment()?,
        client,
        database.client(),
    ));
    let shutdown_downloads = std::sync::Arc::clone(&downloads);

    let application = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_controls::init())
        .manage(database.client())
        .manage(paths)
        .manage(downloads)
        .manage(commands::LaunchSessions::new())
        .setup(|app| {
            commands::downloads::forward_events(
                app.handle().clone(),
                app.state::<std::sync::Arc<licht_core::DownloadManager>>()
                    .inner()
                    .clone(),
            );
            let paths = app.state::<commands::LauncherPaths>();
            let settings =
                licht_core::load_settings(&paths.settings).map_err(|error| error.to_string())?;
            let custom_title_bar = !settings.disable_custom_title_bar;
            app.manage(commands::shell::TitleBarMode {
                custom: custom_title_bar,
            });
            create_main_window(app, custom_title_bar)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::downloads::list_download_jobs,
            commands::downloads::prepare_instance_download,
            commands::downloads::pause_download_job,
            commands::downloads::resume_download_job,
            commands::downloads::cancel_download_job,
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
            commands::instances::list_instance_content,
            commands::instances::set_instance_content_enabled,
            commands::instances::remove_instance_content,
            commands::shell::open_repository,
            commands::shell::custom_title_bar_active,
            commands::shell::restart_app,
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
        .build(tauri::generate_context!())
        .context("Failed to build Licht Launcher")?;
    let mut closing = false;
    let mut database = Some(database);
    application.run(move |app, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event
            && !closing
        {
            closing = true;
            api.prevent_exit();
            let app = app.clone();
            let downloads = std::sync::Arc::clone(&shutdown_downloads);
            let database = database.take();
            tauri::async_runtime::spawn(async move {
                if downloads.suspend_all().await.is_err() {
                    tracing::error!("Failed to persist all download states during shutdown");
                }
                if let Some(database) = database
                    && database.shutdown().await.is_err()
                {
                    tracing::error!("Failed to close the database service during shutdown");
                }
                app.exit(code.unwrap_or(0));
            });
        }
    });
    Ok(())
}

fn create_main_window(app: &tauri::App, custom_title_bar: bool) -> tauri::Result<()> {
    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Licht Launcher")
        .inner_size(1200.0, 800.0)
        .min_inner_size(960.0, 560.0)
        .max_inner_size(1200.0, 960.0)
        .fullscreen(false)
        .maximizable(false)
        .decorations(!custom_title_bar)
        .theme(Some(tauri::Theme::Dark))
        .background_color(tauri::window::Color(0x11, 0x12, 0x14, 0xff));

    if !custom_title_bar {
        builder.build()?;
        return Ok(());
    }

    #[cfg(windows)]
    {
        use tauri_plugin_window_controls::WindowControlsBuilderExt;
        // Native min / (disabled) max / close above the WebView. Maximize stays
        // grayed while `maximizable(false)` clears WS_MAXIMIZEBOX.
        builder.window_controls_height(TITLE_BAR_HEIGHT).build()?;
    }
    #[cfg(not(windows))]
    {
        builder.build()?;
    }

    Ok(())
}
