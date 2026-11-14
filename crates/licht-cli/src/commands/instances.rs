use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use licht_core::{CoreError, InstanceEntry, SharedCache, game_cache, list_instances};

#[derive(Debug, Subcommand)]
pub enum InstancesCommand {
    /// List saved instances.
    List(InstancesListArgs),
}

#[derive(Debug, Args)]
pub struct InstancesListArgs {
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
    /// Print instances as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRef {
    pub folder: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceResolveError {
    Missing,
    Ambiguous { folders: Vec<String> },
}

pub async fn instances_list(args: InstancesListArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let database = licht_core::DatabaseService::start(pool).await?;
    let result = async {
        let root = cache.instances_dir()?;
        let entries = list_instances(&database.client(), &root).await?;
        for line in format_instances_list(
            &entries,
            InstancesListOutput {
                json: args.json,
                interactive: std::io::stdout().is_terminal(),
            },
        ) {
            println!("{line}");
        }
        Ok(0)
    }
    .await;
    database.shutdown().await?;
    result
}

const HEADER_STYLE: &str = "\u{1b}[1;33m";
const DIM: &str = "\u{1b}[90m";
const RESET: &str = "\u{1b}[0m";

#[derive(Debug, Clone, Copy)]
pub struct InstancesListOutput {
    pub json: bool,
    pub interactive: bool,
}

pub fn format_instances_list(
    entries: &[InstanceEntry],
    output: InstancesListOutput,
) -> Vec<String> {
    if output.json {
        vec![format_instances_json(entries)]
    } else if output.interactive {
        format_instances_table(entries, true)
    } else {
        entries.iter().map(instance_tsv_line).collect()
    }
}

fn format_instances_json(entries: &[InstanceEntry]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }

    let mut body = String::from("[\n");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            body.push_str(",\n");
        }
        body.push_str("  {\n");
        body.push_str(&format!(
            "    \"folder\": {},\n",
            json_string(&entry.folder)
        ));
        body.push_str(&format!("    \"name\": {},\n", json_string(&entry.name)));
        match &entry.version_id {
            Some(version) => {
                body.push_str(&format!("    \"versionId\": {}\n", json_string(version)));
            }
            None => body.push_str("    \"versionId\": null\n"),
        }
        body.push_str("  }");
    }
    body.push_str("\n]");
    body
}

fn json_string(value: &str) -> String {
    let mut encoded = String::from('"');
    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{08}' => encoded.push_str("\\b"),
            '\u{0c}' => encoded.push_str("\\f"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            character if character.is_control() => {
                encoded.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => encoded.push(character),
        }
    }
    encoded.push('"');
    encoded
}

fn instance_tsv_line(entry: &InstanceEntry) -> String {
    format!(
        "{}\t{}\t{}",
        entry.folder,
        entry.name,
        instance_version(entry)
    )
}

pub fn format_instances_table(entries: &[InstanceEntry], color: bool) -> Vec<String> {
    if entries.is_empty() {
        return Vec::new();
    }

    const HEADERS: [&str; 3] = ["FOLDER", "NAME", "VERSION"];
    let rows: Vec<[String; 3]> = entries
        .iter()
        .map(|entry| {
            [
                entry.folder.clone(),
                entry.name.clone(),
                instance_version(entry).to_string(),
            ]
        })
        .collect();

    let mut widths = [
        display_width(HEADERS[0]),
        display_width(HEADERS[1]),
        display_width(HEADERS[2]),
    ];
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(display_width(cell));
        }
    }

    let mut lines = Vec::with_capacity(rows.len() + 3);
    lines.push(style_border(
        &format_border_row(widths, '┌', '┬', '┐'),
        color,
    ));
    lines.push(format_header_row(HEADERS, widths, color));
    lines.push(style_border(
        &format_border_row(widths, '├', '┼', '┤'),
        color,
    ));
    for row in &rows {
        lines.push(format_data_row([&row[0], &row[1], &row[2]], widths, color));
    }
    lines.push(style_border(
        &format_border_row(widths, '└', '┴', '┘'),
        color,
    ));
    lines
}

fn format_border_row(widths: [usize; 3], left: char, mid: char, right: char) -> String {
    format!(
        "{left}{folder}{mid}{name}{mid}{version}{right}",
        folder = "─".repeat(widths[0] + 2),
        name = "─".repeat(widths[1] + 2),
        version = "─".repeat(widths[2] + 2),
    )
}

fn format_header_row(headers: [&str; 3], widths: [usize; 3], color: bool) -> String {
    let cells = [
        pad_cell(headers[0], widths[0]),
        pad_cell(headers[1], widths[1]),
        pad_cell(headers[2], widths[2]),
    ];
    if color {
        format_framed_row(
            [
                format!("{HEADER_STYLE}{}{RESET}", cells[0]),
                format!("{HEADER_STYLE}{}{RESET}", cells[1]),
                format!("{HEADER_STYLE}{}{RESET}", cells[2]),
            ],
            true,
        )
    } else {
        format_framed_row(cells, false)
    }
}

fn format_data_row(cells: [&str; 3], widths: [usize; 3], color: bool) -> String {
    let folder = pad_cell(cells[0], widths[0]);
    let name = pad_cell(cells[1], widths[1]);
    let version = pad_cell(cells[2], widths[2]);
    if color {
        format_framed_row([folder, name, format!("{DIM}{version}{RESET}")], true)
    } else {
        format_framed_row([folder, name, version], false)
    }
}

fn format_framed_row(cells: [String; 3], color: bool) -> String {
    if color {
        format!(
            "{DIM}│{RESET} {folder} {DIM}│{RESET} {name} {DIM}│{RESET} {version} {DIM}│{RESET}",
            folder = cells[0],
            name = cells[1],
            version = cells[2],
        )
    } else {
        format!(
            "│ {folder} │ {name} │ {version} │",
            folder = cells[0],
            name = cells[1],
            version = cells[2],
        )
    }
}

fn pad_cell(value: &str, width: usize) -> String {
    format!(
        "{value}{pad}",
        pad = " ".repeat(width.saturating_sub(display_width(value)))
    )
}

fn style_border(line: &str, color: bool) -> String {
    if color {
        format!("{DIM}{line}{RESET}")
    } else {
        line.to_string()
    }
}

fn instance_version(entry: &InstanceEntry) -> &str {
    entry.version_id.as_deref().unwrap_or("-")
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

pub fn instance_refs(entries: &[InstanceEntry]) -> Vec<InstanceRef> {
    entries
        .iter()
        .map(|entry| InstanceRef {
            folder: entry.folder.clone(),
            name: entry.name.clone(),
        })
        .collect()
}

pub fn resolve_instance<'a>(
    query: &str,
    entries: &'a [InstanceRef],
) -> Result<&'a InstanceRef, InstanceResolveError> {
    if let Some(entry) = entries.iter().find(|entry| entry.folder == query) {
        return Ok(entry);
    }

    let matches: Vec<&InstanceRef> = entries.iter().filter(|entry| entry.name == query).collect();
    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => Err(InstanceResolveError::Missing),
        many => Err(InstanceResolveError::Ambiguous {
            folders: many.iter().map(|entry| entry.folder.clone()).collect(),
        }),
    }
}

pub fn resolve_error(query: &str, error: InstanceResolveError) -> CoreError {
    match error {
        InstanceResolveError::Missing => CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("instance `{query}` was not found"),
        )),
        InstanceResolveError::Ambiguous { folders } => {
            CoreError::Io(std::io::Error::other(format!(
                "instance name `{query}` matches multiple folders: {}",
                folders.join(", ")
            )))
        }
    }
}

