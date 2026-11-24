//! The console format of development builds: one line per event, tagged by the area
//! of the launcher that logged it. Events with many fields list them on the lines
//! below instead. Span fields are shown like the event's own fields.

use std::fmt::{self, Write as _};
use std::sync::{Mutex, PoisonError};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Module prefixes and the area they log for, most specific first.
const AREAS: &[(&str, Area)] = &[
    ("licht_launcher::commands::launch", Area::LAUNCH),
    ("licht_launcher::commands::sessions", Area::LAUNCH),
    ("licht_launcher::commands::downloads", Area::DOWNLOAD),
    ("licht_launcher::commands::install_progress", Area::DOWNLOAD),
    ("licht_launcher::commands::accounts", Area::ACCOUNT),
    ("licht_launcher::commands::modpacks", Area::MODPACK),
    ("licht_launcher::commands::instances", Area::INSTANCE),
    ("licht_launcher::commands::versions", Area::VERSION),
    ("licht_launcher::commands::settings", Area::SETTINGS),
    ("licht_launcher::commands::frontend_log", Area::INTERFACE),
    ("licht_launcher::commands", Area::COMMAND),
    ("licht_launcher::app_updates", Area::UPDATE),
    ("licht_launcher::startup::updates", Area::UPDATE),
    ("licht_launcher::startup", Area::STARTUP),
    ("licht_launcher::logging", Area::STARTUP),
    ("licht_core::game", Area::LAUNCH),
    ("licht_core::transfer", Area::DOWNLOAD),
    ("licht_core::runtime", Area::JAVA),
    ("licht_core::updates", Area::UPDATE),
    ("licht_core::auth", Area::ACCOUNT),
    ("licht_core::modpacks", Area::MODPACK),
    ("licht_core::loaders", Area::LOADER),
    ("licht_core::instance", Area::INSTANCE),
    ("licht_core::meta", Area::VERSION),
    ("licht_core::store::settings", Area::SETTINGS),
    ("licht_core::store", Area::STORAGE),
    ("licht_launcher", Area::LAUNCHER),
    ("licht_core", Area::LAUNCHER),
];

/// Wide enough for the longest area label.
const AREA_WIDTH: usize = 9;

/// Where the message starts: time, level and area tag, each followed by a space.
const MESSAGE_COLUMN: usize = 8 + 1 + 5 + 1 + AREA_WIDTH + 2 + 1;

/// Events with more fields than this list them one per line below the message.
const INLINE_FIELDS: usize = 4;

/// Muted colors from the terminal's 256-color palette, so the console stays calm.
mod colors {
    pub(super) const INFO: u8 = 114;
    pub(super) const WARN: u8 = 178;
    pub(super) const ERROR: u8 = 167;
    pub(super) const DEBUG: u8 = 67;
    pub(super) const TRACE: u8 = 139;
    /// Something started or finished well.
    pub(super) const STARTED: u8 = 108;
    pub(super) const STOPPED: u8 = 173;
    pub(super) const EXITED: u8 = 167;
    /// Something about to happen, such as a game launch.
    pub(super) const LAUNCHING: u8 = 222;
    pub(super) const LIBRARY: u8 = 242;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Area {
    pub(super) label: &'static str,
    color: u8,
}

impl Area {
    const STARTUP: Self = Self::new("startup", 109);
    const LAUNCH: Self = Self::new("launch", 73);
    const DOWNLOAD: Self = Self::new("download", 67);
    const JAVA: Self = Self::new("java", 137);
    const UPDATE: Self = Self::new("update", 139);
    const ACCOUNT: Self = Self::new("account", 179);
    const MODPACK: Self = Self::new("modpack", 103);
    const LOADER: Self = Self::new("loader", 97);
    const INSTANCE: Self = Self::new("instance", 72);
    const VERSION: Self = Self::new("version", 110);
    const SETTINGS: Self = Self::new("settings", 144);
    const INTERFACE: Self = Self::new("interface", 168);
    const STORAGE: Self = Self::new("storage", 102);
    const COMMAND: Self = Self::new("command", 247);
    const LAUNCHER: Self = Self::new("launcher", 247);

    const fn new(label: &'static str, color: u8) -> Self {
        Self { label, color }
    }
}

/// The launcher area a module logs for. Libraries are labeled with their crate name.
pub(super) fn area(target: &str) -> Area {
    AREAS
        .iter()
        .find(|(prefix, _)| {
            target
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
        })
        .map(|(_, area)| *area)
        .unwrap_or_else(|| Area {
            label: library_name(target),
            color: colors::LIBRARY,
        })
}

/// Static, so library labels need no allocation per event.
fn library_name(target: &str) -> &'static str {
    match target.split("::").next() {
        Some("hyper" | "hyper_util") => "hyper",
        Some("reqwest") => "reqwest",
        Some("sqlx") => "sqlx",
        Some("tauri") => "tauri",
        Some("tao") => "tao",
        Some("wry") => "wry",
        _ => "library",
    }
}

/// An ANSI text style. `Plain` keeps the terminal's own text color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Dim,
    Color(u8),
    BoldColor(u8),
}

fn level_style(level: Level) -> Style {
    match level {
        Level::ERROR => Style::BoldColor(colors::ERROR),
        Level::WARN => Style::Color(colors::WARN),
        Level::INFO => Style::Color(colors::INFO),
        Level::DEBUG => Style::Color(colors::DEBUG),
        Level::TRACE => Style::Color(colors::TRACE),
    }
}

/// Warnings and errors take their level's color; otherwise only messages about
/// something launching, starting, completing, stopping or exiting stand out.
fn message_style(level: Level, message: &str) -> Style {
    match level {
        Level::ERROR => return Style::Color(colors::ERROR),
        Level::WARN => return Style::Color(colors::WARN),
        _ => {}
    }
    message
        .split(|character: char| !character.is_alphanumeric())
        .find_map(|word| match word.to_ascii_lowercase().as_str() {
            "started" | "starting" | "completed" => Some(Style::Color(colors::STARTED)),
            "stopped" | "stopping" => Some(Style::Color(colors::STOPPED)),
            "exited" => Some(Style::Color(colors::EXITED)),
            "launching" => Some(Style::Color(colors::LAUNCHING)),
            _ => None,
        })
        .unwrap_or(Style::Plain)
}

/// Writes `text` in `style`, with escapes only when `ansi` is set.
fn paint(out: &mut impl fmt::Write, ansi: bool, style: Style, text: &str) -> fmt::Result {
    let code = match style {
        Style::Plain => return out.write_str(text),
        _ if !ansi => return out.write_str(text),
        Style::Dim => "2".to_string(),
        Style::Color(color) => format!("38;5;{color}"),
        Style::BoldColor(color) => format!("1;38;5;{color}"),
    };
    write!(out, "\u{1b}[{code}m{text}\u{1b}[0m")
}

/// Returns the cursor to the start of the line and erases it.
const CLEAR_LINE: &str = "\r\u{1b}[2K";

/// The message apart from the other fields. Strings keep no quotes, so values read
/// as plain text.
#[derive(Default)]
struct EventFields {
    message: String,
    fields: Vec<(&'static str, String)>,
}

impl Visit for EventFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push((field.name(), value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push((field.name(), format!("{value:?}")));
        }
    }
}

const PROGRESS_BAR_WIDTH: u64 = 20;
const BYTES_PER_MB: f64 = 1024.0 * 1024.0;

/// The numbers of a `Download progress` event, taken out of its fields.
#[derive(Debug, Default, PartialEq, Eq)]
struct Progress {
    percent: u64,
    finished: u64,
    total: u64,
    bytes_done: u64,
    bytes_total: u64,
}

impl Progress {
    /// Removes the numeric progress fields, leaving the rest to be shown as usual.
    fn from_fields(fields: &mut Vec<(&'static str, String)>) -> Self {
        let mut progress = Self::default();
        fields.retain(|(name, value)| {
            let slot = match *name {
                "percent" => &mut progress.percent,
                "finished" => &mut progress.finished,
                "total" => &mut progress.total,
                "bytes_done" => &mut progress.bytes_done,
                "bytes_total" => &mut progress.bytes_total,
                _ => return true,
            };
            match value.parse() {
                Ok(number) => {
                    *slot = number;
                    false
                }
                Err(_) => true,
            }
        });
        progress
    }

    /// `[████░░░░]  40%  824/2061 files  96.4/241.0 MB`; counts appear when known.
    fn write(&self, out: &mut impl fmt::Write, ansi: bool, color: u8) -> fmt::Result {
        let percent = self.percent.min(100);
        let filled = percent * PROGRESS_BAR_WIDTH / 100;
        let filled_width = usize::try_from(filled).unwrap_or_default();
        let empty_width = usize::try_from(PROGRESS_BAR_WIDTH - filled).unwrap_or_default();
        out.write_str("[")?;
        paint(out, ansi, Style::Color(color), &"█".repeat(filled_width))?;
        paint(out, ansi, Style::Dim, &"░".repeat(empty_width))?;
        write!(out, "] {percent:>3}%")?;
        if self.total > 0 {
            write!(out, "  {}/{} files", self.finished, self.total)?;
        }
        if self.bytes_total > 0 {
            // Display only, so the precision lost converting to f64 does not matter.
            let done = self.bytes_done as f64 / BYTES_PER_MB;
            let total = self.bytes_total as f64 / BYTES_PER_MB;
            write!(out, "  {done:.1}/{total:.1} MB")?;
        }
        Ok(())
    }
}

fn write_inline(
    out: &mut impl fmt::Write,
    ansi: bool,
    fields: &[(&'static str, String)],
) -> fmt::Result {
    for (name, value) in fields {
        out.write_str(" ")?;
        paint(out, ansi, Style::Dim, &format!("{name}="))?;
        out.write_str(value)?;
    }
    Ok(())
}

/// Whether a download message means its job no longer reports progress.
fn ends_progress(message: &str) -> bool {
    match message.strip_prefix("Download job ") {
        Some(rest) => !matches!(rest, "started" | "resumed"),
        None => message == "All downloads paused",
    }
}

/// The fields of a span, recorded by [`SpanFieldsLayer`] so events can show them.
struct SpanFields(Vec<(&'static str, String)>);

/// Records span fields the same way as event fields, unquoted and by name.
pub(super) struct SpanFieldsLayer;

impl<S> Layer<S> for SpanFieldsLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let Some(span) = context.span(id) else {
            return;
        };
        let mut fields = EventFields::default();
        attributes.record(&mut fields);
        span.extensions_mut().insert(SpanFields(fields.fields));
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, context: Context<'_, S>) {
        let Some(span) = context.span(id) else {
            return;
        };
        let mut fields = EventFields::default();
        values.record(&mut fields);
        if let Some(recorded) = span.extensions_mut().get_mut::<SpanFields>() {
            recorded.0.extend(fields.fields);
        }
    }
}

/// `HH:MM:SS level [area] message fields`, with the UTC time of the log file. The
/// fields of the event's spans follow its own. Events with many fields put each
/// field on its own line instead.
///
/// With colors on, download progress stays on one line at the bottom: each update
/// redraws it, and other events are printed above it.
#[derive(Default)]
pub(super) struct DevelopmentFormat {
    /// The progress line currently drawn at the bottom of the console.
    pinned: Mutex<Option<String>>,
}

impl<S, N> FormatEvent<S, N> for DevelopmentFormat
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let ansi = writer.has_ansi_escapes();
        let metadata = event.metadata();
        let mut line = String::new();
        let mut stamp = String::new();
        SystemTime.format_time(&mut format::Writer::new(&mut stamp))?;
        paint(
            &mut line,
            ansi,
            Style::Dim,
            stamp.get(11..19).unwrap_or(&stamp),
        )?;
        line.push(' ');
        let level = *metadata.level();
        paint(&mut line, ansi, level_style(level), &format!("{level:<5}"))?;
        line.push(' ');
        let area = area(metadata.target());
        let tag = format!("[{}]", area.label);
        paint(
            &mut line,
            ansi,
            Style::Color(area.color),
            &format!("{tag:<width$}", width = AREA_WIDTH + 2),
        )?;
        let mut fields = EventFields::default();
        event.record(&mut fields);
        line.push(' ');
        paint(
            &mut line,
            ansi,
            message_style(level, &fields.message),
            &fields.message,
        )?;
        let mut all_fields = fields.fields;
        if let Some(scope) = context.event_scope() {
            for span in scope.from_root() {
                if let Some(span_fields) = span.extensions().get::<SpanFields>() {
                    all_fields.extend(span_fields.0.iter().cloned());
                }
            }
        }

        let mut pinned = self.pinned.lock().unwrap_or_else(PoisonError::into_inner);
        if metadata.target() == licht_core::DOWNLOAD_PROGRESS_TARGET {
            let progress = Progress::from_fields(&mut all_fields);
            line.push(' ');
            progress.write(&mut line, ansi, area.color)?;
            write_inline(&mut line, ansi, &all_fields)?;
            if !ansi {
                return writeln!(writer, "{line}");
            }
            write!(writer, "{CLEAR_LINE}{line}")?;
            if progress.percent >= 100 {
                *pinned = None;
                return writeln!(writer);
            }
            *pinned = Some(line);
            return Ok(());
        }

        if all_fields.len() <= INLINE_FIELDS {
            write_inline(&mut line, ansi, &all_fields)?;
            line.push('\n');
        } else {
            line.push('\n');
            let width = all_fields
                .iter()
                .map(|(name, _)| name.len())
                .max()
                .unwrap_or_default();
            for (name, value) in &all_fields {
                write!(line, "{:MESSAGE_COLUMN$}", "")?;
                paint(&mut line, ansi, Style::Dim, &format!("{name:<width$}"))?;
                writeln!(line, "  {value}")?;
            }
        }
        if pinned.is_some() {
            writer.write_str(CLEAR_LINE)?;
        }
        writer.write_str(&line)?;
        if ends_progress(&fields.message) {
            *pinned = None;
        }
        if let Some(progress) = pinned.as_deref() {
            writer.write_str(progress)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::{AREAS, Progress, Style, area, colors, ends_progress, library_name, message_style};

    #[test]
    fn job_endings_stop_the_pinned_progress_line() {
        for message in [
            "Download job completed",
            "Download job failed",
            "Download job cancelled",
            "Download job paused",
            "All downloads paused",
        ] {
            assert!(ends_progress(message), "{message}");
        }
        for message in [
            "Download job started",
            "Download job resumed",
            "Game started",
        ] {
            assert!(!ends_progress(message), "{message}");
        }
    }

    #[test]
    fn progress_numbers_are_taken_out_of_the_fields() {
        let mut fields = vec![
            ("percent", "40".to_string()),
            ("finished", "824".to_string()),
            ("total", "2061".to_string()),
            ("bytes_done", "101082726".to_string()),
            ("bytes_total", "252706816".to_string()),
            ("component", "Minecraft".to_string()),
            ("job_id", "7".to_string()),
        ];

        let progress = Progress::from_fields(&mut fields);

        assert_eq!(
            progress,
            Progress {
                percent: 40,
                finished: 824,
                total: 2061,
                bytes_done: 101_082_726,
                bytes_total: 252_706_816,
            }
        );
        assert_eq!(
            fields,
            [
                ("component", "Minecraft".to_string()),
                ("job_id", "7".to_string())
            ]
        );
    }

    #[test]
    fn modules_are_tagged_with_the_area_they_log_for() {
        for (target, label) in [
            ("licht_launcher::commands::launch", "launch"),
            ("licht_core::game::run", "launch"),
            ("licht_core::transfer::manager::worker", "download"),
            ("licht_launcher::app_updates", "update"),
            ("licht_launcher::startup::updates", "update"),
            ("licht_launcher::startup::state", "startup"),
            ("licht_launcher::logging", "startup"),
            ("licht_core::store::settings", "settings"),
            ("licht_core::store::layout::migration", "storage"),
            ("licht_launcher::commands", "command"),
            ("licht_launcher::commands::frontend_log", "interface"),
            ("licht_launcher", "launcher"),
            ("licht_launcher::desktop", "launcher"),
        ] {
            assert_eq!(area(target).label, label, "{target}");
        }
    }

    #[test]
    fn a_prefix_matches_whole_module_names_only() {
        assert_eq!(area("licht_core::gameplay").label, "launcher");
        assert_eq!(area("licht_launcher_extra").label, "library");
    }

    #[test]
    fn libraries_are_labeled_with_their_crate() {
        assert_eq!(area("hyper_util::client::legacy").label, "hyper");
        assert_eq!(area("sqlx::query").label, "sqlx");
        assert_eq!(library_name("mio::poll"), "library");
        assert_eq!(area("mio::poll").color, colors::LIBRARY);
    }

    #[test]
    fn specific_modules_come_before_their_parents() {
        for (index, (earlier, _)) in AREAS.iter().enumerate() {
            for (later, _) in &AREAS[index + 1..] {
                assert!(
                    !later.starts_with(&format!("{earlier}::")),
                    "{later} must come before {earlier}"
                );
            }
        }
    }

    #[test]
    fn starting_and_stopping_messages_get_their_own_colors() {
        for message in [
            "Game started",
            "Download job started",
            "Starting Licht Launcher",
        ] {
            assert_eq!(
                message_style(Level::INFO, message),
                Style::Color(colors::STARTED),
                "{message}"
            );
        }
        for message in ["Game stopped by the launcher", "Stopping downloads"] {
            assert_eq!(
                message_style(Level::INFO, message),
                Style::Color(colors::STOPPED),
                "{message}"
            );
        }
        assert_eq!(
            message_style(Level::INFO, "Download job completed"),
            Style::Color(colors::STARTED)
        );
        assert_eq!(
            message_style(Level::INFO, "Game exited"),
            Style::Color(colors::EXITED)
        );
        assert_eq!(
            message_style(Level::INFO, "Launching game"),
            Style::Color(colors::LAUNCHING)
        );
        assert_eq!(
            message_style(Level::INFO, "Launcher is up to date"),
            Style::Plain
        );
    }

    #[test]
    fn only_whole_words_mark_a_start_or_stop() {
        assert_eq!(
            message_style(Level::INFO, "Download job restarted as a full repair"),
            Style::Plain
        );
    }

    #[test]
    fn warnings_and_errors_keep_their_level_color() {
        assert_eq!(
            message_style(Level::WARN, "Game stopped unexpectedly"),
            Style::Color(colors::WARN)
        );
        assert_eq!(
            message_style(Level::ERROR, "Startup worker stopped"),
            Style::Color(colors::ERROR)
        );
    }
}
