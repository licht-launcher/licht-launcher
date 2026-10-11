use crate::{CoreError, Result};

/// Environment variable that holds the Azure Application (client) ID.
pub const MSA_CLIENT_ID_ENV: &str = "LICHT_MSA_CLIENT_ID";

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
    use super::*;

    #[test]
    fn missing_env_is_an_error() {
        // SAFETY: tests in this crate run single-threaded for this env key.
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
        let error = msa_client_id().expect_err("missing");
        assert!(matches!(error, CoreError::MsaClientIdMissing));
    }

    #[test]
    fn blank_env_is_an_error() {
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
        unsafe {
            std::env::set_var(MSA_CLIENT_ID_ENV, "  test-client-id  ");
        }
        let id = msa_client_id().expect("id");
        assert_eq!(id, "test-client-id");
        unsafe {
            std::env::remove_var(MSA_CLIENT_ID_ENV);
        }
    }
}
