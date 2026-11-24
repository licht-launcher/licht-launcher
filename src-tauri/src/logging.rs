use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, filter::LevelFilter, fmt, reload};

const LOG_DIRECTORY: &str = "logs";
const LOG_FILE_PREFIX: &str = "licht";
const LOG_FILE_SUFFIX: &str = "log";
const RETAINED_LOG_FILES: usize = 30;
/// Debug events from the launcher's own crates; libraries stay at INFO.
const VERBOSE_DIRECTIVES: &str = "info,licht_core=debug,licht_launcher=debug";

/// Switches verbose logging while the launcher runs. A `RUST_LOG` filter, when set,
/// always wins.
pub(crate) struct LogLevel {
    filter: Option<reload::Handle<EnvFilter, Registry>>,
    verbose: AtomicBool,
}

impl LogLevel {
    pub(crate) fn set_verbose(&self, verbose: bool) -> Result<()> {
        let Some(filter) = &self.filter else {
            return Ok(());
        };
        if self.verbose.swap(verbose, Ordering::SeqCst) == verbose {
            return Ok(());
        }
        filter
            .reload(level_filter(None, verbose)?)
            .context("Could not change the log level")?;
        tracing::info!(verbose, "Verbose logging changed");
        Ok(())
    }
}

pub(crate) fn init(verbose: bool) -> Result<LogLevel> {
    let directives = match env::var("RUST_LOG") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("RUST_LOG must contain valid Unicode"),
    };

    let (filter, handle) = reload::Layer::new(level_filter(directives.as_deref(), verbose)?);
    let opened = log_directory().and_then(|directory| {
        let file = open_log_file(&directory, &launch_stamp()?)?;
        Ok((directory, file))
    });
    let (directory, file, file_error) = match opened {
        Ok((directory, file)) => (Some(directory), Some(file), None),
        Err(error) => (None, None, Some(error)),
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
    if let Some(directory) = directory
        && let Err(error) = prune_old_logs(&directory, RETAINED_LOG_FILES)
    {
        tracing::warn!(%error, "Could not remove old launcher log files");
    }
    install_panic_hook();
    Ok(LogLevel {
        filter: directives.is_none().then_some(handle),
        verbose: AtomicBool::new(verbose),
    })
}

/// The folder holding the launcher log files, inside the profile directory.
pub(crate) fn log_directory() -> Result<PathBuf> {
    let cache = licht_core::SharedCache::system()
        .context("Could not resolve the launcher data directory")?;
    Ok(cache.root().join(LOG_DIRECTORY))
}

fn parse_filter(directives: Option<&str>) -> Result<EnvFilter> {
    EnvFilter::builder()
        .with_regex(false)
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives.unwrap_or(""))
        .context("Invalid RUST_LOG filter")
}

fn level_filter(directives: Option<&str>, verbose: bool) -> Result<EnvFilter> {
    parse_filter(directives.or(verbose.then_some(VERBOSE_DIRECTIVES)))
}

/// UTC launch time such as `2026-11-23T15-30-12Z`: sortable and valid in file names.
fn launch_stamp() -> Result<String> {
    let mut text = String::new();
    SystemTime
        .format_time(&mut Writer::new(&mut text))
        .map_err(|_| anyhow::anyhow!("Could not format the launch time"))?;
    let seconds = text
        .get(..19)
        .context("The launch time has an unexpected format")?;
    Ok(format!("{}Z", seconds.replace(':', "-")))
}

/// Each launch writes its own file, named after the time it started.
fn open_log_file(directory: &Path, stamp: &str) -> Result<File> {
    fs::create_dir_all(directory)
        .and_then(|()| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join(format!("{LOG_FILE_PREFIX}.{stamp}.{LOG_FILE_SUFFIX}")))
        })
        .context("Could not open the launcher log file")
}

/// Removes all but the newest `keep` launcher logs. Names sort by launch time, and
/// older daily files (`licht.<date>.log`) sort before launches from the same day.
fn prune_old_logs(directory: &Path, keep: usize) -> io::Result<()> {
    let prefix = format!("{LOG_FILE_PREFIX}.");
    let suffix = format!(".{LOG_FILE_SUFFIX}");
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str()
            && name.starts_with(&prefix)
            && name.ends_with(&suffix)
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    let excess = names.len().saturating_sub(keep);
    for name in &names[..excess] {
        fs::remove_file(directory.join(name))?;
    }
    Ok(())
}

fn subscriber(
    filter: impl Layer<Registry> + Send + Sync,
    file: Option<File>,
) -> impl Subscriber + Send + Sync {
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
        .with(file.map(|file| fmt::layer().with_writer(file).with_ansi(false)))
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
    use std::path::{Path, PathBuf};

    use licht_testkit::logs::LogCapture;
    use licht_testkit::temp::TempDir;

    use std::sync::atomic::AtomicBool;

    use tracing_subscriber::reload;

    use super::{
        LogLevel, install_panic_hook, launch_stamp, level_filter, open_log_file, parse_filter,
        prune_old_logs, subscriber,
    };

    fn capture_logs_while(directives: Option<&str>, emit: impl FnOnce()) -> String {
        let capture =
            LogCapture::filtered(parse_filter(directives).expect("the filter should be valid"));
        emit();
        capture.text()
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

    fn emit_levels() {
        tracing::debug!("launcher debug event");
        tracing::debug!(target: "hyper::client", "library debug event");
        tracing::info!(target: "hyper::client", "library info event");
    }

    #[test]
    fn verbose_logging_adds_debug_events_from_the_launcher_only() {
        let capture = LogCapture::filtered(level_filter(None, true).expect("verbose filter"));
        emit_levels();
        let logs = capture.text();

        assert!(
            logs.contains("launcher debug event"),
            "unexpected logs: {logs}"
        );
        assert!(
            logs.contains("library info event"),
            "unexpected logs: {logs}"
        );
        assert!(
            !logs.contains("library debug event"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn rust_log_wins_over_verbose_logging() {
        let capture =
            LogCapture::filtered(level_filter(Some("warn"), true).expect("configured filter"));
        emit_levels();
        let logs = capture.text();

        assert!(logs.is_empty(), "unexpected logs: {logs}");
    }

    #[test]
    fn verbose_logging_can_be_switched_while_running() {
        let root = TempDir::new("logging-test");
        let file = open_log_file(root.path(), "2026-11-24T10-00-00Z").expect("log file");
        let (filter, handle) =
            reload::Layer::new(level_filter(None, false).expect("default filter"));
        let level = LogLevel {
            filter: Some(handle),
            verbose: AtomicBool::new(false),
        };
        // Callsite interest is shared between threads; see `LogCapture` in licht-testkit.
        let _registered = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());

        tracing::subscriber::with_default(subscriber(filter, Some(file)), || {
            tracing::debug!("before switching");
            level.set_verbose(true).expect("enable verbose logging");
            tracing::debug!("while verbose");
            level
                .set_verbose(true)
                .expect("enable verbose logging again");
            level.set_verbose(false).expect("disable verbose logging");
            tracing::debug!("after switching back");
        });

        let logs = fs::read_to_string(root.path().join("licht.2026-11-24T10-00-00Z.log"))
            .expect("the log file should be readable");
        assert!(
            !logs.contains("before switching"),
            "unexpected logs: {logs}"
        );
        assert!(logs.contains("while verbose"), "unexpected logs: {logs}");
        assert!(
            !logs.contains("after switching back"),
            "unexpected logs: {logs}"
        );
        assert_eq!(
            logs.matches("Verbose logging changed").count(),
            2,
            "unexpected logs: {logs}"
        );
    }

    fn file_names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = log_files(directory)
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .expect("the log file name should be valid UTF-8")
                    .to_string()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn each_launch_writes_its_own_file_in_a_created_directory() {
        let root = TempDir::new("logging-test");
        let directory = root.path().join("logs");

        for (stamp, message) in [
            ("2026-11-23T09-00-00Z", "first launch"),
            ("2026-11-23T15-30-12Z", "second launch"),
        ] {
            let file = open_log_file(&directory, stamp).expect("the log file should open");
            let filter = parse_filter(None).expect("the filter should be valid");
            tracing::subscriber::with_default(subscriber(filter, Some(file)), || {
                tracing::debug!("debug event");
                tracing::info!(launch = message, "startup event");
            });
        }

        assert_eq!(
            file_names(&directory),
            [
                "licht.2026-11-23T09-00-00Z.log",
                "licht.2026-11-23T15-30-12Z.log"
            ]
        );
        let second = fs::read_to_string(directory.join("licht.2026-11-23T15-30-12Z.log"))
            .expect("the log file should be readable");
        assert!(second.contains("second launch"));
        assert!(!second.contains("first launch"));
        assert!(!second.contains("debug event"));
        assert!(
            !second.contains('\u{1b}'),
            "file logs must not contain ANSI codes"
        );
    }

    #[test]
    fn file_logs_report_an_error_when_the_directory_cannot_be_created() {
        let root = TempDir::new("logging-test");
        let blocker = root.path().join("not-a-directory");
        fs::write(&blocker, b"").expect("the blocking file should be written");

        let error = open_log_file(&blocker.join("logs"), "2026-11-23T15-30-12Z")
            .expect_err("a log directory below a file should fail");

        assert_eq!(error.to_string(), "Could not open the launcher log file");
        assert!(error.source().is_some());
    }

    #[test]
    fn pruning_keeps_only_the_newest_launcher_logs() {
        let root = TempDir::new("logging-test");
        let directory = root.path();
        fs::write(directory.join("licht.2026-11-20.log"), b"").expect("legacy log");
        for minute in 0..33 {
            fs::write(
                directory.join(format!("licht.2026-11-23T10-{minute:02}-00Z.log")),
                b"",
            )
            .expect("launch log");
        }
        fs::write(directory.join("notes.txt"), b"").expect("unrelated file");

        prune_old_logs(directory, 30).expect("old logs should be removed");

        let names = file_names(directory);
        assert_eq!(names.len(), 31, "unexpected files: {names:?}");
        assert!(names.contains(&"notes.txt".to_string()));
        assert!(!names.contains(&"licht.2026-11-20.log".to_string()));
        assert!(!names.contains(&"licht.2026-11-23T10-02-00Z.log".to_string()));
        assert!(names.contains(&"licht.2026-11-23T10-03-00Z.log".to_string()));
        assert!(names.contains(&"licht.2026-11-23T10-32-00Z.log".to_string()));
    }

    #[test]
    fn launch_stamps_are_sortable_and_safe_in_file_names() {
        let stamp = launch_stamp().expect("the launch time should format");

        assert_eq!(stamp.len(), 20, "unexpected stamp {stamp}");
        assert!(stamp.ends_with('Z'), "unexpected stamp {stamp}");
        for (index, character) in stamp.char_indices() {
            let expected_separator = match index {
                4 | 7 | 13 | 16 => Some('-'),
                10 => Some('T'),
                19 => Some('Z'),
                _ => None,
            };
            match expected_separator {
                Some(separator) => assert_eq!(character, separator, "unexpected stamp {stamp}"),
                None => assert!(character.is_ascii_digit(), "unexpected stamp {stamp}"),
            }
        }
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
