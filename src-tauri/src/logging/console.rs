//! The console format of development builds: each event is tagged and colored by the
//! area of the launcher that logged it, with its fields and spans on the lines below.

use std::fmt;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormattedFields};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Area {
    pub(super) label: &'static str,
    /// A color from the terminal's 256-color palette.
    color: u8,
}

impl Area {
    const LAUNCH: Self = Self::new("launch", 44);
    const DOWNLOAD: Self = Self::new("download", 33);
    const JAVA: Self = Self::new("java", 172);
    const UPDATE: Self = Self::new("update", 170);
    const ACCOUNT: Self = Self::new("account", 214);
    const MODPACK: Self = Self::new("modpack", 141);
    const LOADER: Self = Self::new("loader", 135);
    const INSTANCE: Self = Self::new("instance", 79);
    const VERSION: Self = Self::new("version", 110);
    const SETTINGS: Self = Self::new("settings", 180);
    const INTERFACE: Self = Self::new("interface", 204);
    const STORAGE: Self = Self::new("storage", 109);
    const COMMAND: Self = Self::new("command", 247);
    const LAUNCHER: Self = Self::new("launcher", 245);
    const LIBRARY_COLOR: u8 = 240;

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
            color: Area::LIBRARY_COLOR,
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

/// An ANSI SGR style, such as `1` (bold) or `38;5;44` (a 256-color foreground).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    Bold,
    Dim,
    /// What the standard pretty format uses for `at`, `in` and `with`.
    DimItalic,
    Red,
    BoldRed,
    Green,
    Yellow,
    Blue,
    Magenta,
    Palette(u8),
}

impl fmt::Display for Style {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bold => formatter.write_str("1"),
            Self::Dim => formatter.write_str("2"),
            Self::DimItalic => formatter.write_str("2;3"),
            Self::Red => formatter.write_str("31"),
            Self::BoldRed => formatter.write_str("1;31"),
            Self::Green => formatter.write_str("32"),
            Self::Yellow => formatter.write_str("33"),
            Self::Blue => formatter.write_str("34"),
            Self::Magenta => formatter.write_str("35"),
            Self::Palette(color) => write!(formatter, "38;5;{color}"),
        }
    }
}

fn level_style(level: Level) -> Style {
    match level {
        Level::ERROR => Style::BoldRed,
        Level::WARN => Style::Yellow,
        Level::INFO => Style::Green,
        Level::DEBUG => Style::Blue,
        Level::TRACE => Style::Magenta,
    }
}

/// Writes `text` in `style` when the writer accepts escapes.
fn paint(writer: &mut format::Writer<'_>, style: Style, text: &str) -> fmt::Result {
    if writer.has_ansi_escapes() {
        write!(writer, "\u{1b}[{style}m{text}\u{1b}[0m")
    } else {
        writer.write_str(text)
    }
}

/// Where the message starts, so the lines below it line up: time, level and area.
const DETAIL_INDENT: usize = 8 + 1 + 5 + 1 + AREA_WIDTH + 2 + 1;

/// The message apart from the other fields, which go on their own lines.
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
            self.record_debug(field, &value);
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

/// `HH:MM:SS level [area] message` in UTC, like the log file, then indented lines in
/// the style of the standard pretty format: one per field, the source location, and
/// one per span. Span fields come from the layer's field formatter, which development
/// builds set to `PrettyFields`.
pub(super) struct DevelopmentFormat;

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
        let metadata = event.metadata();
        let mut stamp = String::new();
        SystemTime.format_time(&mut format::Writer::new(&mut stamp))?;
        paint(&mut writer, Style::Dim, stamp.get(11..19).unwrap_or(&stamp))?;
        write!(writer, " ")?;
        let level = *metadata.level();
        paint(&mut writer, level_style(level), &format!("{level:<5}"))?;
        write!(writer, " ")?;
        let area = area(metadata.target());
        let tag = format!("[{}]", area.label);
        paint(
            &mut writer,
            Style::Palette(area.color),
            &format!("{tag:<width$}", width = AREA_WIDTH + 2),
        )?;
        let mut fields = EventFields::default();
        event.record(&mut fields);
        let message_style = match level {
            Level::ERROR => Style::Red,
            Level::WARN => Style::Yellow,
            _ => Style::Bold,
        };
        write!(writer, " ")?;
        paint(&mut writer, message_style, &fields.message)?;
        writeln!(writer)?;
        let indent = " ".repeat(DETAIL_INDENT);
        for (name, value) in &fields.fields {
            write!(writer, "{indent}")?;
            paint(&mut writer, Style::Bold, name)?;
            writeln!(writer, ": {value}")?;
        }
        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            write!(writer, "{indent}")?;
            paint(&mut writer, Style::DimItalic, "at")?;
            writeln!(writer, " {file}:{line}")?;
        }
        if let Some(scope) = context.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{indent}")?;
                paint(&mut writer, Style::DimItalic, "in")?;
                write!(writer, " ")?;
                paint(&mut writer, Style::Bold, span.name())?;
                let extensions = span.extensions();
                if let Some(span_fields) = extensions
                    .get::<FormattedFields<N>>()
                    .filter(|span_fields| !span_fields.is_empty())
                {
                    write!(writer, " ")?;
                    paint(&mut writer, Style::DimItalic, "with")?;
                    write!(writer, " {span_fields}")?;
                }
                writeln!(writer)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AREAS, Area, area};

    #[test]
    fn modules_are_tagged_with_the_area_they_log_for() {
        for (target, label) in [
            ("licht_launcher::commands::launch", "launch"),
            ("licht_core::game::run", "launch"),
            ("licht_core::transfer::manager::worker", "download"),
            ("licht_launcher::app_updates", "update"),
            ("licht_core::store::settings", "settings"),
            ("licht_core::store::layout::migration", "storage"),
            ("licht_launcher::commands", "command"),
            ("licht_launcher::commands::frontend_log", "interface"),
            ("licht_launcher", "launcher"),
            ("licht_launcher::logging", "launcher"),
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
        assert_eq!(area("mio::poll").label, "library");
        assert_eq!(area("mio::poll").color, Area::LIBRARY_COLOR);
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
}
