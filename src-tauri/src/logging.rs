use std::env;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::Subscriber;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, filter::LevelFilter, fmt};

const LOG_DIRECTORY: &str = "logs";
const LOG_FILE_PREFIX: &str = "licht";
const LOG_FILE_SUFFIX: &str = "log";
const RETAINED_LOG_FILES: usize = 7;

pub(crate) fn init() -> Result<()> {
    let directives = match env::var("RUST_LOG") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("RUST_LOG must contain valid Unicode"),
    };

    let filter = parse_filter(directives.as_deref())?;
    let file = licht_core::SharedCache::system()
        .context("Could not resolve the launcher data directory")
        .and_then(|cache| file_appender(&cache.root().join(LOG_DIRECTORY)));
    let (file, file_error) = match file {
        Ok(appender) => (Some(appender), None),
        Err(error) => (None, Some(error)),
    };

    subscriber(filter, file)
        .try_init()
        .context("Failed to initialize logging")?;

    if let Some(error) = file_error {
        tracing::warn!(
            error = format!("{error:#}"),
            "Could not open the launcher log file; logging to stderr only"
        );
    }
    install_panic_hook();
    Ok(())
}

fn parse_filter(directives: Option<&str>) -> Result<EnvFilter> {
    EnvFilter::builder()
        .with_regex(false)
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives.unwrap_or(""))
        .context("Invalid RUST_LOG filter")
}

fn file_appender(directory: &Path) -> Result<RollingFileAppender> {
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(RETAINED_LOG_FILES)
        .build(directory)
        .context("Could not open the launcher log file")
}

fn subscriber(
    filter: EnvFilter,
    file: Option<RollingFileAppender>,
) -> impl Subscriber + Send + Sync {
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
        .with(file.map(|appender| fmt::layer().with_writer(appender).with_ansi(false)))
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "Launcher panicked");
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{file_appender, install_panic_hook, parse_filter, subscriber};

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

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "licht-logging-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("the test directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn capture_logs_while(directives: Option<&str>, emit: impl FnOnce()) -> String {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer_output = output.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(parse_filter(directives).expect("the filter should be valid"))
            .with_writer(move || TestWriter(writer_output.clone()))
            .with_ansi(false)
            .without_time()
            .finish();

        tracing::subscriber::with_default(subscriber, emit);

        let bytes = output
            .lock()
            .expect("the log buffer lock should work")
            .clone();
        String::from_utf8(bytes).expect("logs should contain valid UTF-8")
    }

    fn capture_logs(directives: Option<&str>) -> String {
        capture_logs_while(directives, || {
            tracing::debug!("debug event");
            tracing::info!(version = "0.1.1", "startup event");
            tracing::warn!("warning event");
        })
    }

    fn log_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .expect("the log directory should exist")
            .map(|entry| entry.expect("the log entry should be readable").path())
            .collect()
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

    #[test]
    fn file_logs_are_written_to_a_dated_file_in_a_created_directory() {
        let root = TempDirectory::new();
        let directory = root.path().join("logs");
        let appender = file_appender(&directory).expect("the log file should open");
        let filter = parse_filter(None).expect("the filter should be valid");

        tracing::subscriber::with_default(subscriber(filter, Some(appender)), || {
            tracing::debug!("debug event");
            tracing::info!(version = "0.1.1", "startup event");
        });

        let files = log_files(&directory);
        assert_eq!(files.len(), 1);
        let name = files[0]
            .file_name()
            .and_then(|name| name.to_str())
            .expect("the log file name should be valid UTF-8");
        assert!(name.starts_with("licht."), "unexpected log file {name}");
        assert!(name.ends_with(".log"), "unexpected log file {name}");
        let contents = fs::read_to_string(&files[0]).expect("the log file should be readable");
        assert!(contents.contains("startup event"));
        assert!(contents.contains("version=\"0.1.1\""));
        assert!(!contents.contains("debug event"));
        assert!(
            !contents.contains('\u{1b}'),
            "file logs must not contain ANSI codes"
        );
    }

    #[test]
    fn file_logs_report_an_error_when_the_directory_cannot_be_created() {
        let root = TempDirectory::new();
        let blocker = root.path().join("not-a-directory");
        fs::write(&blocker, b"").expect("the blocking file should be written");

        let error = file_appender(&blocker.join("logs"))
            .expect_err("a log directory below a file should fail");

        assert_eq!(error.to_string(), "Could not open the launcher log file");
        assert!(error.source().is_some());
    }

    #[test]
    fn panics_are_logged_as_errors() {
        install_panic_hook();
        let logs = capture_logs_while(None, || {
            let result = std::panic::catch_unwind(|| panic!("boom from a test"));
            assert!(result.is_err());
        });
        drop(std::panic::take_hook());

        assert!(logs.contains("ERROR"));
        assert!(logs.contains("Launcher panicked"));
        assert!(logs.contains("boom from a test"));
    }
}
