use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, filter::LevelFilter, fmt};

mod console;

const LOG_DIRECTORY: &str = "logs";
const LOG_FILE_PREFIX: &str = "licht";
const LOG_FILE_SUFFIX: &str = "log";
const RETAINED_LOG_FILES: usize = 30;

pub(crate) fn init() -> Result<()> {
    let directives = match env::var("RUST_LOG") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("RUST_LOG must contain valid Unicode"),
    };

    let filter = parse_filter(directives.as_deref())?;
    let opened = log_directory().and_then(|directory| {
        let file = open_log_file(&directory, &launch_stamp()?)?;
        Ok((directory, file))
    });
    let (directory, file, file_error) = match opened {
        Ok((directory, file)) => (Some(directory), Some(file), None),
        Err(error) => (None, None, Some(error)),
    };

    subscriber(filter, ConsoleStyle::detect(), file)
        .try_init()
        .context("Failed to initialize logging")?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Licht Launcher"
    );

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
    Ok(())
}

pub(crate) fn log_system_information(diagnostics: &licht_core::BuildDiagnostics) {
    tracing::info!(
        version = %diagnostics.host_version,
        revision = diagnostics.host_revision.as_deref(),
        debug = diagnostics.debug,
        tauri = %diagnostics.tauri_version,
        webview = diagnostics.webview_version.as_deref(),
        os = %diagnostics.os_name,
        os_version = diagnostics.os_version.as_deref(),
        kernel = diagnostics.kernel_version.as_deref(),
        system_architecture = %diagnostics.system_architecture,
        process_architecture = %diagnostics.process_architecture,
        "System information"
    );
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

/// How stderr shows events. The log file always keeps the standard format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConsoleStyle {
    /// Events tagged by launcher area, for development builds.
    development: bool,
    colors: bool,
}

impl ConsoleStyle {
    fn detect() -> Self {
        Self::for_build(
            cfg!(debug_assertions),
            io::stderr().is_terminal(),
            env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
        )
    }

    /// `tauri dev` pipes the app's stderr into its own terminal, so development
    /// builds use colors even when stderr is not a terminal. `NO_COLOR` always wins.
    fn for_build(debug: bool, terminal: bool, no_color: bool) -> Self {
        Self {
            development: debug,
            colors: !no_color && (debug || terminal),
        }
    }
}

fn console_layer<S>(
    style: ConsoleStyle,
    writer: impl for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
) -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let layer = fmt::layer().with_writer(writer).with_ansi(style.colors);
    if style.development {
        layer.event_format(console::DevelopmentFormat).boxed()
    } else {
        layer.boxed()
    }
}

fn subscriber(
    filter: EnvFilter,
    console: ConsoleStyle,
    file: Option<File>,
) -> impl Subscriber + Send + Sync {
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer(console, std::io::stderr))
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

    use tracing_subscriber::layer::SubscriberExt;

    use super::{
        ConsoleStyle, console_layer, install_panic_hook, launch_stamp, log_system_information,
        open_log_file, parse_filter, prune_old_logs, subscriber,
    };

    #[test]
    fn development_builds_color_the_console_even_through_a_pipe() {
        assert_eq!(
            ConsoleStyle::for_build(true, false, false),
            ConsoleStyle {
                development: true,
                colors: true
            }
        );
        assert_eq!(
            ConsoleStyle::for_build(false, true, false),
            ConsoleStyle {
                development: false,
                colors: true
            }
        );
        assert_eq!(
            ConsoleStyle::for_build(false, false, false),
            ConsoleStyle {
                development: false,
                colors: false
            }
        );
    }

    #[test]
    fn no_color_turns_colors_off_in_every_build() {
        for (debug, terminal) in [(true, true), (true, false), (false, true)] {
            assert!(!ConsoleStyle::for_build(debug, terminal, true).colors);
        }
    }

    fn console_output(style: ConsoleStyle) -> String {
        console_output_of(style, || {
            let _launch = tracing::info_span!("launch", instance = "Survival").entered();
            tracing::info!(
                target: "licht_launcher::commands::launch",
                pid = 18432,
                "Game started"
            );
        })
    }

    fn console_output_of(style: ConsoleStyle, emit: impl FnOnce()) -> String {
        let root = TempDir::new("logging-test");
        let path = root.path().join("console.log");
        let file = fs::File::create(&path).expect("the console file should be created");
        let console = tracing_subscriber::registry().with(console_layer(style, file));
        tracing::subscriber::with_default(console, emit);
        fs::read_to_string(path).expect("the console output should be readable")
    }

    #[test]
    fn the_development_console_lists_many_fields_below_the_message() {
        let output = console_output_of(ConsoleStyle::for_build(true, false, true), || {
            tracing::info!(
                target: "licht_launcher::logging",
                version = "0.3.1",
                os = "Windows 11 Pro",
                debug = true,
                system_architecture = "x86_64",
                kernel = 26300,
                "System information"
            );
        });
        let lines: Vec<_> = output.lines().collect();
        let indent = " ".repeat(27);

        assert!(
            lines[0].ends_with(" INFO  [startup]   System information"),
            "unexpected output: {output}"
        );
        assert_eq!(
            lines[1..],
            [
                format!("{indent}version              0.3.1"),
                format!("{indent}os                   Windows 11 Pro"),
                format!("{indent}debug                true"),
                format!("{indent}system_architecture  x86_64"),
                format!("{indent}kernel               26300"),
            ]
        );
    }

    #[test]
    fn the_development_console_tags_events_with_their_area_in_color() {
        let output = console_output(ConsoleStyle::for_build(true, false, false));

        assert_eq!(output.lines().count(), 1, "unexpected output: {output}");
        assert!(output.contains("[launch]"), "the area is missing: {output}");
        assert!(
            output.contains("Game started"),
            "unexpected output: {output}"
        );
        assert!(output.contains("pid"), "unexpected output: {output}");
        assert!(
            output.contains("instance") && output.contains("\"Survival\""),
            "the span fields are missing: {output}"
        );
        assert!(output.contains('\u{1b}'), "colors are missing: {output}");
    }

    #[test]
    fn the_development_console_has_no_escapes_without_colors() {
        let output = console_output(ConsoleStyle::for_build(true, false, true));

        assert_eq!(output.lines().count(), 1, "unexpected output: {output}");
        assert!(
            output.ends_with(" INFO  [launch]    Game started pid=18432 instance=\"Survival\"\n"),
            "unexpected output: {output}"
        );
        assert!(!output.contains('\u{1b}'), "unexpected colors: {output}");
    }

    #[test]
    fn the_release_console_keeps_one_plain_line_per_event() {
        let output = console_output(ConsoleStyle::for_build(false, false, false));

        assert_eq!(output.lines().count(), 1, "unexpected output: {output}");
        assert!(!output.contains('\u{1b}'), "unexpected colors: {output}");
        assert!(
            output.contains("launch{instance=\"Survival\"}"),
            "unexpected output: {output}"
        );
    }

    #[test]
    fn system_information_is_logged_on_one_line() {
        let capture = LogCapture::start();
        log_system_information(&licht_core::BuildDiagnostics {
            host_version: "0.3.1".to_string(),
            host_revision: Some("48520a7".to_string()),
            debug: false,
            tauri_version: "2.9.5".to_string(),
            webview_version: None,
            os_name: "Windows 11 Pro".to_string(),
            os_version: Some("10.0.26300".to_string()),
            kernel_version: Some("26300".to_string()),
            system_architecture: "x86_64".to_string(),
            process_architecture: "x86_64".to_string(),
        });
        let logs = capture.text();

        assert_eq!(logs.lines().count(), 1, "unexpected logs: {logs}");
        assert!(logs.contains("INFO"), "unexpected logs: {logs}");
        assert!(
            logs.contains("System information"),
            "unexpected logs: {logs}"
        );
        for field in [
            "version=0.3.1",
            "revision=\"48520a7\"",
            "os=Windows 11 Pro",
            "os_version=\"10.0.26300\"",
            "process_architecture=x86_64",
        ] {
            assert!(logs.contains(field), "missing {field}: {logs}");
        }
        assert!(!logs.contains("webview="), "unexpected logs: {logs}");
    }

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
            let console = ConsoleStyle::for_build(false, false, true);
            tracing::subscriber::with_default(subscriber(filter, console, Some(file)), || {
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
