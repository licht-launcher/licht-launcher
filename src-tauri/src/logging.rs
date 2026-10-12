use std::env;

use anyhow::{Context, Result};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

pub(crate) fn init() -> Result<()> {
    let directives = match env::var("RUST_LOG") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("RUST_LOG must contain valid Unicode"),
    };

    let filter = parse_filter(directives.as_deref())?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init()
        .map_err(anyhow::Error::from_boxed)
        .context("Failed to initialize logging")
}

fn parse_filter(directives: Option<&str>) -> Result<EnvFilter> {
    EnvFilter::builder()
        .with_regex(false)
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives.unwrap_or(""))
        .context("Invalid RUST_LOG filter")
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    use super::parse_filter;

    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .expect("the log buffer lock should work")
                .write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn capture_logs(directives: Option<&str>) -> String {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer_output = output.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(parse_filter(directives).expect("the filter should be valid"))
            .with_writer(move || TestWriter(writer_output.clone()))
            .with_ansi(false)
            .without_time()
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("debug event");
            tracing::info!(version = "0.1.1", "startup event");
            tracing::warn!("warning event");
        });

        let bytes = output
            .lock()
            .expect("the log buffer lock should work")
            .clone();
        String::from_utf8(bytes).expect("logs should contain valid UTF-8")
    }

    #[test]
    fn default_filter_emits_info_and_warn_but_not_debug() {
        let logs = capture_logs(None);

        assert!(logs.contains("startup event"));
        assert!(logs.contains("version=\"0.1.1\""));
        assert!(logs.contains("warning event"));
        assert!(!logs.contains("debug event"));
    }

    #[test]
    fn configured_filter_changes_the_emitted_levels() {
        let logs = capture_logs(Some("warn"));

        assert!(logs.contains("warning event"));
        assert!(!logs.contains("startup event"));
        assert!(!logs.contains("debug event"));
    }

    #[test]
    fn invalid_directives_return_an_error_with_context() {
        let error = parse_filter(Some("licht_launcher=invalid"))
            .expect_err("the filter should reject invalid levels");

        assert_eq!(error.to_string(), "Invalid RUST_LOG filter");
        assert!(error.source().is_some());
    }
}
