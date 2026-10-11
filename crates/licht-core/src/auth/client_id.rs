use std::path::PathBuf;
use std::sync::Once;

use crate::{CoreError, Result};

/// Environment variable that overrides the embedded Azure Application (client) ID.
pub const MSA_CLIENT_ID_ENV: &str = "LICHT_MSA_CLIENT_ID";

/// Public OAuth client ID baked into every build (not a secret).
///
/// Runtime `LICHT_MSA_CLIENT_ID` / `.env` overrides this for local forks and tests.
pub const EMBEDDED_MSA_CLIENT_ID: &str = "e750bdc1-545a-430a-b795-975886cbaa67";

static ENV_FILES_LOADED: Once = Once::new();

/// Loads `.env` from the process working directory and next to the executable.
///
/// Existing process environment variables are not overwritten. Missing files are ignored.
/// Call once at process startup (GUI / CLI). Tests should not call this.
pub fn load_env_files() {
    ENV_FILES_LOADED.call_once(|| {
        let _ = dotenvy::dotenv();
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let path = PathBuf::from(dir).join(".env");
            let _ = dotenvy::from_path(path);
        }
    });
}

/// Resolves the Microsoft application client ID.
///
/// Order: non-empty process env (including values loaded from `.env`) → embedded release ID.
/// The value is never logged.
pub fn msa_client_id() -> Result<String> {
    match std::env::var(MSA_CLIENT_ID_ENV) {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
        Err(_) => {}
    }
    if EMBEDDED_MSA_CLIENT_ID.is_empty() {
        Err(CoreError::MsaClientIdMissing)
    } else {
        Ok(EMBEDDED_MSA_CLIENT_ID.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn falls_back_to_the_embedded_client_id() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        // SAFETY: serialized by ENV_LOCK for this process env key.
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
        let id = msa_client_id().expect("embedded");
        assert_eq!(id, EMBEDDED_MSA_CLIENT_ID);
    }

    #[test]
    fn blank_env_falls_back_to_embedded() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var(MSA_CLIENT_ID_ENV, "  ");
        }
        let id = msa_client_id().expect("embedded");
        assert_eq!(id, EMBEDDED_MSA_CLIENT_ID);
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
    }

    #[test]
    fn runtime_env_overrides_the_embedded_client_id() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var(MSA_CLIENT_ID_ENV, "  test-client-id  ");
        }
        let id = msa_client_id().expect("id");
        assert_eq!(id, "test-client-id");
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
    }

    #[test]
    fn dotenv_file_can_override_the_embedded_client_id() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = std::env::temp_dir().join(format!(
            "licht-dotenv-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join(".env");
        std::fs::write(&path, format!("{MSA_CLIENT_ID_ENV}=from-dotenv-file\n")).expect("write");
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
        dotenvy::from_path(&path).expect("load");
        let id = msa_client_id().expect("id");
        assert_eq!(id, "from-dotenv-file");
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
