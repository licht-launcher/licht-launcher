use anyhow::Context;
use tauri::{WebviewUrl, WebviewWindowBuilder};

mod commands;
mod logging;
mod progress;
mod startup;

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

    let runtime = startup::state::StartupRuntime::new();
    let startup_runtime = std::sync::Arc::clone(&runtime);

    let application = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_controls::init())
        .manage(std::sync::Arc::clone(&runtime))
        .manage(commands::LaunchSessions::new())
        .setup(move |app| {
            startup::start(app.handle().clone(), startup_runtime)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup::commands::get_startup_progress,
            startup::commands::open_startup_preview,
            startup::commands::restart_startup_preview,
            startup::commands::request_update_restart,
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
            commands::shell::open_feedback,
            commands::shell::custom_title_bar_active,
            commands::shell::restart_app,
            commands::instances::get_last_played_folder,
            commands::instances::set_last_played_folder,
            commands::instances::touch_last_opened,
            commands::instances::clear_last_played_folder,
            commands::accounts::list_microsoft_accounts,
            commands::accounts::active_microsoft_account,
            commands::accounts::get_player_head,
            commands::accounts::begin_microsoft_login,
            commands::accounts::set_active_microsoft_account,
            commands::accounts::use_offline_account,
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
    application.run(move |app, event| {
        if let tauri::RunEvent::ExitRequested { ref api, code, .. } = event
            && !runtime
                .shutdown_complete
                .load(std::sync::atomic::Ordering::SeqCst)
        {
            api.prevent_exit();
            if closing {
                return;
            }
            closing = true;
            let app = app.clone();
            let runtime = std::sync::Arc::clone(&runtime);
            tauri::async_runtime::spawn(async move {
                runtime.shutdown().await;
                runtime
                    .shutdown_complete
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                app.exit(code.unwrap_or(0));
            });
        } else if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Destroyed,
            ..
        } = event
            && label.starts_with("startup-preview-")
            && let Ok(mut sessions) = runtime.sessions.lock()
            && let Some(session) = sessions.remove(&label)
        {
            session.stop();
        }
    });
    Ok(())
}

fn create_main_window(
    app: &tauri::AppHandle,
    custom_title_bar: bool,
    transparency: bool,
) -> tauri::Result<()> {
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

    #[cfg(windows)]
    let builder = if transparency {
        builder.transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .effects(
            tauri::window::EffectsBuilder::new()
                .effect(tauri::utils::WindowEffect::Acrylic)
                .color(tauri::window::Color(0x11, 0x12, 0x14, 125))
                .build(),
        )
        .initialization_script(
            "document.addEventListener('DOMContentLoaded', () => { document.documentElement.dataset.windowEffect = 'acrylic'; }, { once: true });",
        )
    } else {
        builder
    };

    #[cfg(not(windows))]
    let _ = transparency;

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
