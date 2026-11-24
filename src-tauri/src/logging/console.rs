//! The console format of development builds: one line per event, tagged by the area
//! of the launcher that logged it. Events with many fields list them on the lines
//! below instead. Span fields are shown like the event's own fields.

use std::fmt::{self, Write as _};
use std::sync::{Mutex, PoisonError};

use time::{OffsetDateTime, UtcOffset};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
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

/// Events with more fields of their own than this list them, together with their
/// span fields, one per line below the message. Span fields alone do not, except for
/// download job lifecycle events, which always list their job's details.
const INLINE_FIELDS: usize = 4;

/// Standard ANSI foreground colors (30-37 and their bright 90-97 variants), so the
/// console follows the terminal theme like Vite, Node and the Tauri CLI do.
mod colors {
    pub(super) const INFO: u8 = 32;
    pub(super) const WARN: u8 = 33;
    pub(super) const ERROR: u8 = 31;
    pub(super) const DEBUG: u8 = 34;
    pub(super) const TRACE: u8 = 35;
    /// Something launched, started or finished well.
    pub(super) const STARTED: u8 = 32;
    pub(super) const STOPPED: u8 = 33;
    pub(super) const EXITED: u8 = 31;
    pub(super) const LIBRARY: u8 = 90;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Area {
    pub(super) label: &'static str,
    color: u8,
}

impl Area {
    const STARTUP: Self = Self::new("startup", 36);
    const LAUNCH: Self = Self::new("launch", 96);
    const DOWNLOAD: Self = Self::new("download", 94);
    const JAVA: Self = Self::new("java", 93);
    const UPDATE: Self = Self::new("update", 95);
    const ACCOUNT: Self = Self::new("account", 33);
    const MODPACK: Self = Self::new("modpack", 35);
    const LOADER: Self = Self::new("loader", 34);
    const INSTANCE: Self = Self::new("instance", 32);
    const VERSION: Self = Self::new("version", 36);
    const SETTINGS: Self = Self::new("settings", 37);
    const INTERFACE: Self = Self::new("interface", 91);
    const STORAGE: Self = Self::new("storage", 34);
    const COMMAND: Self = Self::new("command", 90);
    const LAUNCHER: Self = Self::new("launcher", 90);

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
    /// A standard ANSI foreground code from [`colors`] or an [`Area`].
    Color(u8),
    BoldColor(u8),
}

/// Levels and area tags are bold, so the start of each line stands out.
fn level_style(level: Level) -> Style {
    Style::BoldColor(match level {
        Level::ERROR => colors::ERROR,
        Level::WARN => colors::WARN,
        Level::INFO => colors::INFO,
        Level::DEBUG => colors::DEBUG,
        Level::TRACE => colors::TRACE,
    })
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
            "launching" | "started" | "starting" | "completed" => {
                Some(Style::Color(colors::STARTED))
            }
            "stopped" | "stopping" => Some(Style::Color(colors::STOPPED)),
            "exited" => Some(Style::Color(colors::EXITED)),
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
        Style::Color(color) => color.to_string(),
        Style::BoldColor(color) => format!("1;{color}"),
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

    /// `[████░░░░]  40%`, always `PROGRESS_BAR_WIDTH + 7` columns wide.
    fn write_bar(&self, out: &mut impl fmt::Write, ansi: bool, color: u8) -> fmt::Result {
        let percent = self.percent.min(100);
        let filled = percent * PROGRESS_BAR_WIDTH / 100;
        let filled_width = usize::try_from(filled).unwrap_or_default();
        let empty_width = usize::try_from(PROGRESS_BAR_WIDTH - filled).unwrap_or_default();
        out.write_str("[")?;
        paint(out, ansi, Style::Color(color), &"█".repeat(filled_width))?;
        paint(out, ansi, Style::Dim, &"░".repeat(empty_width))?;
        write!(out, "] {percent:>3}%")
    }

    /// Megabytes when the size is known, files otherwise.
    fn counts(&self) -> Option<String> {
        if self.bytes_total > 0 {
            // Display only, so the precision lost converting to f64 does not matter.
            let done = self.bytes_done as f64 / BYTES_PER_MB;
            let total = self.bytes_total as f64 / BYTES_PER_MB;
            Some(format!("{done:.1}/{total:.1} MB"))
        } else if self.total > 0 {
            Some(format!("{}/{} files", self.finished, self.total))
        } else {
            None
        }
    }
}

/// A redrawn line must fit on one terminal row: a longer line wraps, and returning
/// to its start would only clear the last row, leaving the rest behind.
const PROGRESS_LINE_LIMIT: usize = 100;

/// `component phase [bar] percent counts instance` after the usual prefix. Counts and
/// the instance are left out when the line would grow past `PROGRESS_LINE_LIMIT`.
fn progress_line(
    prefix: &str,
    progress: &Progress,
    fields: &[(&'static str, String)],
    ansi: bool,
    color: u8,
) -> Result<String, fmt::Error> {
    let field = |wanted: &str| {
        fields
            .iter()
            .find(|(name, _)| *name == wanted)
            .map(|(_, value)| value.as_str())
    };
    let mut line = prefix.to_string();
    let mut width = MESSAGE_COLUMN - 1;
    if let Some(component) = field("component") {
        write!(line, " {component}")?;
        width += 1 + component.chars().count();
    }
    if let Some(phase) = field("phase") {
        let phase = phase.to_ascii_lowercase();
        line.push(' ');
        paint(&mut line, ansi, Style::Dim, &phase)?;
        width += 1 + phase.chars().count();
    }
    line.push_str("  ");
    progress.write_bar(&mut line, ansi, color)?;
    width += 2 + usize::try_from(PROGRESS_BAR_WIDTH).unwrap_or_default() + 7;
    let optional = [
        (progress.counts(), Style::Plain),
        (field("instance").map(str::to_string), Style::Dim),
    ];
    for (text, style) in optional {
        let Some(text) = text else {
            continue;
        };
        let text_width = 2 + text.chars().count();
        if width + text_width > PROGRESS_LINE_LIMIT {
            break;
        }
        line.push_str("  ");
        paint(&mut line, ansi, style, &text)?;
        width += text_width;
    }
    Ok(line)
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

/// `HH:MM:SS level [area] message fields`, with the local time of day. The
/// fields of the event's spans follow its own. Events with many fields put each
/// field on its own line instead.
///
/// With colors on, download progress stays on one line at the bottom: each update
/// redraws it, and other events are printed above it.
pub(super) struct DevelopmentFormat {
    /// The progress line currently drawn at the bottom of the console.
    pinned: Mutex<Option<String>>,
    utc_offset: UtcOffset,
}

impl DevelopmentFormat {
    /// Shows the time with `utc_offset`, normally the machine's local offset.
    pub(super) fn new(utc_offset: UtcOffset) -> Self {
        Self {
            pinned: Mutex::default(),
            utc_offset,
        }
    }
}

/// `HH:MM:SS` of `now` at `utc_offset`.
fn clock(now: OffsetDateTime, utc_offset: UtcOffset) -> String {
    let local = now.to_offset(utc_offset);
    format!(
        "{:02}:{:02}:{:02}",
        local.hour(),
        local.minute(),
        local.second()
    )
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
        let stamp = clock(OffsetDateTime::now_utc(), self.utc_offset);
        paint(&mut line, ansi, Style::Dim, &stamp)?;
        line.push(' ');
        let level = *metadata.level();
        paint(&mut line, ansi, level_style(level), &format!("{level:<5}"))?;
        line.push(' ');
        let area = area(metadata.target());
        let tag = format!("[{}]", area.label);
        paint(
            &mut line,
            ansi,
            Style::BoldColor(area.color),
            &format!("{tag:<width$}", width = AREA_WIDTH + 2),
        )?;
        let prefix = line.clone();
        let mut fields = EventFields::default();
        event.record(&mut fields);
        line.push(' ');
        paint(
            &mut line,
            ansi,
            message_style(level, &fields.message),
            &fields.message,
        )?;
        let listed =
            fields.fields.len() > INLINE_FIELDS || fields.message.starts_with("Download job ");
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
            let line = progress_line(&prefix, &progress, &all_fields, ansi, area.color)?;
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

        if !listed {
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

    use time::{OffsetDateTime, UtcOffset};

    use super::{
        AREAS, PROGRESS_LINE_LIMIT, Progress, Style, area, clock, colors, ends_progress,
        level_style, library_name, message_style, progress_line,
    };

    #[test]
    fn every_level_is_bold_in_its_color() {
        for (level, color) in [
            (Level::ERROR, colors::ERROR),
            (Level::WARN, colors::WARN),
            (Level::INFO, colors::INFO),
            (Level::DEBUG, colors::DEBUG),
            (Level::TRACE, colors::TRACE),
        ] {
            assert_eq!(level_style(level), Style::BoldColor(color), "{level}");
        }
    }

    #[test]
    fn the_clock_shows_the_given_offset() {
        let utc = |seconds| {
            OffsetDateTime::from_unix_timestamp(seconds).expect("the timestamp should be valid")
        };
        let brasilia = UtcOffset::from_hms(-3, 0, 0).expect("the offset should be valid");

        // 2026-11-24T19:50:38Z and 2026-11-24T01:05:09Z.
        assert_eq!(clock(utc(1_795_549_838), UtcOffset::UTC), "19:50:38");
        assert_eq!(clock(utc(1_795_549_838), brasilia), "16:50:38");
        assert_eq!(clock(utc(1_795_482_309), brasilia), "22:05:09");
    }

    #[test]
    fn the_progress_line_leaves_out_what_does_not_fit() {
        let progress = Progress {
            percent: 39,
            finished: 2048,
            total: 5214,
            ..Progress::default()
        };
        let prefix = "19:50:38 INFO  [download] ";
        let fields = |instance: &str| {
            vec![
                ("component", "Minecraft".to_string()),
                ("phase", "Verifying".to_string()),
                ("instance", instance.to_string()),
            ]
        };

        let short = progress_line(prefix, &progress, &fields("S1"), false, 67)
            .expect("the line should format");
        assert_eq!(
            short,
            "19:50:38 INFO  [download]  Minecraft verifying  [███████░░░░░░░░░░░░░]  39%  2048/5214 files  S1"
        );

        let long = progress_line(prefix, &progress, &fields(&"x".repeat(40)), false, 67)
            .expect("the line should format");
        assert!(long.ends_with("2048/5214 files"), "{long}");
        assert!(long.chars().count() <= PROGRESS_LINE_LIMIT, "{long}");
    }

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
            Style::Color(colors::STARTED)
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
