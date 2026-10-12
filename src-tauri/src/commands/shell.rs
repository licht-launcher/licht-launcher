use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

const REPOSITORY_URL: &str = "https://github.com/mateusneiva/licht-launcher";

#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|error| error.to_string())
}
