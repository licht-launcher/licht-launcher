//! Links the UI asks to open in the system browser.

use url::Url;

use crate::{CoreError, Result};

/// Accepts only absolute HTTPS URLs without embedded credentials, so a page
/// cannot use the browser hand-off to run `file:`, `javascript:` or custom
/// protocol handlers.
pub fn external_https_url(raw: &str) -> Result<String> {
    let url = Url::parse(raw.trim()).map_err(|_| CoreError::ExternalUrl)?;
    let has_credentials = !url.username().is_empty() || url.password().is_some();
    if url.scheme() != "https" || url.host_str().is_none() || has_credentials {
        return Err(CoreError::ExternalUrl);
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_links() {
        assert_eq!(
            external_https_url(" https://modrinth.com/modpack/fo ").expect("https"),
            "https://modrinth.com/modpack/fo"
        );
    }

    #[test]
    fn rejects_other_schemes_relative_links_and_credentials() {
        for raw in [
            "http://example.com",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "steam://run/1",
            "/relative",
            "https://user:secret@example.com",
        ] {
            assert!(external_https_url(raw).is_err(), "{raw}");
        }
    }
}
