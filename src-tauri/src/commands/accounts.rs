use licht_core::{
    AccountProfile, get_active_profile, list_accounts, login_with_browser, logout_account,
    set_active_account,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use super::{LauncherPaths, failure, http_client};

#[tauri::command]
pub async fn list_microsoft_accounts(
    database: State<'_, licht_core::DatabaseClient>,
) -> Result<Vec<AccountProfile>, String> {
    list_accounts(&database).await.map_err(failure)
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
    database: State<'_, licht_core::DatabaseClient>,
) -> Result<AccountProfile, String> {
    let client = http_client().map_err(failure)?;
    let opener = app.clone();
    login_with_browser(&client, &database, move |url| {
        opener
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(failure)
}

#[tauri::command]
pub async fn set_active_microsoft_account(
    database: State<'_, licht_core::DatabaseClient>,
    uuid: String,
) -> Result<AccountProfile, String> {
    set_active_account(&database, &uuid).await.map_err(failure)
}

#[tauri::command]
pub async fn logout_microsoft_account(
    database: State<'_, licht_core::DatabaseClient>,
    uuid: String,
) -> Result<(), String> {
    logout_account(&database, &uuid).await.map_err(failure)
}
