use licht_core::{
    AccountProfile, get_active_profile, list_accounts, login_with_browser, logout_account,
    set_active_account,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use super::{LauncherPaths, failure, http_client};

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
pub async fn begin_microsoft_login(
    app: AppHandle,
    paths: State<'_, LauncherPaths>,
) -> Result<AccountProfile, String> {
    let client = http_client().map_err(failure)?;
    let opener = app.clone();
    login_with_browser(&client, &paths.settings, move |url| {
        opener
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| error.to_string())
    })
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
