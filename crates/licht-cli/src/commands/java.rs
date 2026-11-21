use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use licht_core::{JavaDetections, SharedCache, detect_known_javas, game_cache};

use crate::host::host_environment;

#[derive(Debug, Subcommand)]
pub enum JavaCommand {
    /// List Java runtimes found on this machine and in the launcher cache.
    Detect(JavaDetectArgs),
}

#[derive(Debug, Args)]
pub struct JavaDetectArgs {
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
    /// Print detections as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct JavaDetectOutput {
    pub json: bool,
    pub interactive: bool,
}

pub async fn java_detect(args: JavaDetectArgs) -> licht_core::Result<i32> {
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&licht_core::initialize_launcher().await?).await?,
    };
    let environment = host_environment()?;
    let detections = detect_known_javas(&cache, environment.os);
    for line in format_java_detections(
        &detections,
        JavaDetectOutput {
            json: args.json,
            interactive: std::io::stdout().is_terminal(),
        },
    ) {
        println!("{line}");
    }
    Ok(0)
}

pub fn format_java_detections(
    detections: &JavaDetections,
    output: JavaDetectOutput,
) -> Vec<String> {
    if output.json {
        vec![format_java_json(detections)]
    } else if output.interactive {
        format_java_table(detections)
    } else {
        java_rows(detections)
            .into_iter()
            .map(|(major, path)| format!("{major}\t{path}"))
            .collect()
    }
}

fn java_rows(detections: &JavaDetections) -> Vec<(u32, &str)> {
    let mut rows = Vec::new();
    for path in &detections.java25 {
        rows.push((25, path.as_str()));
    }
    for path in &detections.java21 {
        rows.push((21, path.as_str()));
    }
    for path in &detections.java17 {
        rows.push((17, path.as_str()));
    }
    for path in &detections.java8 {
        rows.push((8, path.as_str()));
    }
    rows
}

fn format_java_json(detections: &JavaDetections) -> String {
    let mut body = String::from("{\n");
    body.push_str(&format!(
        "  \"java25\": {},\n",
        json_string_array(&detections.java25)
    ));
    body.push_str(&format!(
        "  \"java21\": {},\n",
        json_string_array(&detections.java21)
    ));
    body.push_str(&format!(
        "  \"java17\": {},\n",
        json_string_array(&detections.java17)
    ));
    body.push_str(&format!(
        "  \"java8\": {}\n",
        json_string_array(&detections.java8)
    ));
    body.push('}');
    body
}

fn json_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }
    let mut body = String::from("[\n");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            body.push_str(",\n");
        }
        body.push_str(&format!("    {}", json_string(value)));
    }
    body.push_str("\n  ]");
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

fn format_java_table(detections: &JavaDetections) -> Vec<String> {
    let rows = java_rows(detections);
    if rows.is_empty() {
        return Vec::new();
    }

    const HEADERS: [&str; 2] = ["MAJOR", "PATH"];
    let rendered: Vec<[String; 2]> = rows
        .iter()
        .map(|(major, path)| [major.to_string(), (*path).to_string()])
        .collect();

    let mut widths = [display_width(HEADERS[0]), display_width(HEADERS[1])];
    for row in &rendered {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(display_width(cell));
        }
    }

    let mut lines = Vec::with_capacity(rendered.len() + 1);
    lines.push(format!(
        "{}  {}",
        pad_cell(HEADERS[0], widths[0]),
        pad_cell(HEADERS[1], widths[1])
    ));
    for row in &rendered {
        lines.push(format!(
            "{}  {}",
            pad_cell(&row[0], widths[0]),
            pad_cell(&row[1], widths[1])
        ));
    }
    lines
}

fn pad_cell(value: &str, width: usize) -> String {
    format!(
        "{value}{pad}",
        pad = " ".repeat(width.saturating_sub(display_width(value)))
    )
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

#[cfg(test)]
mod tests {
    use super::{JavaDetectOutput, format_java_detections};
    use licht_core::JavaDetections;

    fn sample() -> JavaDetections {
        JavaDetections {
            java25: Vec::new(),
            java21: vec![r"C:\Java\jdk-21\bin\java.exe".into()],
            java17: vec![r"C:\Java\jdk-17\bin\java.exe".into()],
            java8: Vec::new(),
        }
    }

    #[test]
    fn non_interactive_list_uses_tabs() {
        assert_eq!(
            format_java_detections(
                &sample(),
                JavaDetectOutput {
                    json: false,
                    interactive: false,
                }
            ),
            vec![
                "21\tC:\\Java\\jdk-21\\bin\\java.exe".to_string(),
                "17\tC:\\Java\\jdk-17\\bin\\java.exe".to_string(),
            ]
        );
    }

    #[test]
    fn interactive_list_prints_aligned_columns() {
        assert_eq!(
            format_java_detections(
                &sample(),
                JavaDetectOutput {
                    json: false,
                    interactive: true,
                }
            ),
            vec![
                "MAJOR  PATH                       ".to_string(),
                "21     C:\\Java\\jdk-21\\bin\\java.exe".to_string(),
                "17     C:\\Java\\jdk-17\\bin\\java.exe".to_string(),
            ]
        );
    }

    #[test]
    fn json_list_groups_paths_by_major() {
        assert_eq!(
            format_java_detections(
                &sample(),
                JavaDetectOutput {
                    json: true,
                    interactive: false,
                }
            ),
            vec![
                "{\n  \"java25\": [],\n  \"java21\": [\n    \"C:\\\\Java\\\\jdk-21\\\\bin\\\\java.exe\"\n  ],\n  \"java17\": [\n    \"C:\\\\Java\\\\jdk-17\\\\bin\\\\java.exe\"\n  ],\n  \"java8\": []\n}"
                    .to_string()
            ]
        );
    }

    #[test]
    fn interactive_list_is_empty_when_nothing_was_found() {
        assert!(
            format_java_detections(
                &JavaDetections::default(),
                JavaDetectOutput {
                    json: false,
                    interactive: true,
                }
            )
            .is_empty()
        );
    }
}
