use std::path::PathBuf;
use std::sync::Once;

use crate::{CoreError, Result};

/// Environment variable that holds the Azure Application (client) ID.
pub const MSA_CLIENT_ID_ENV: &str = "LICHT_MSA_CLIENT_ID";

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

/// Reads the Microsoft application client ID from the environment.
///
/// The value is never logged. It must not be committed to the repository.
pub fn msa_client_id() -> Result<String> {
    match std::env::var(MSA_CLIENT_ID_ENV) {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                Err(CoreError::MsaClientIdMissing)
            } else {
                Ok(trimmed)
            }
        }
        Err(_) => Err(CoreError::MsaClientIdMissing),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn missing_env_is_an_error() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        // SAFETY: serialized by ENV_LOCK for this process env key.
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
        let error = msa_client_id().expect_err("missing");
        assert!(matches!(error, CoreError::MsaClientIdMissing));
    }

    #[test]
    fn blank_env_is_an_error() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var(MSA_CLIENT_ID_ENV, "  ");
        }
        let error = msa_client_id().expect_err("blank");
        assert!(matches!(error, CoreError::MsaClientIdMissing));
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
    }

    #[test]
    fn trims_and_returns_the_client_id() {
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
    fn dotenv_file_can_supply_the_client_id() {
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
