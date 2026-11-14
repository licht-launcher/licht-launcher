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
        format_instances_table(entries)
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

fn format_instances_table(entries: &[InstanceEntry]) -> Vec<String> {
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

    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(format_table_row(HEADERS[0], HEADERS[1], HEADERS[2], widths));
    lines.push(format!(
        "{}  {}  {}",
        "─".repeat(widths[0]),
        "─".repeat(widths[1]),
        "─".repeat(widths[2]),
    ));
    for row in &rows {
        lines.push(format_table_row(&row[0], &row[1], &row[2], widths));
    }
    lines
}

fn format_table_row(folder: &str, name: &str, version: &str, widths: [usize; 3]) -> String {
    format!(
        "{folder}{folder_pad}  {name}{name_pad}  {version}",
        folder_pad = " ".repeat(widths[0].saturating_sub(display_width(folder))),
        name_pad = " ".repeat(widths[1].saturating_sub(display_width(name))),
    )
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

#[cfg(test)]
mod tests {
    use super::{
        InstanceRef, InstanceResolveError, InstancesListOutput, format_instances_list,
        resolve_instance,
    };
    use licht_core::InstanceEntry;

    fn auto(interactive: bool) -> InstancesListOutput {
        InstancesListOutput {
            json: false,
            interactive,
        }
    }

    fn entry(folder: &str, name: &str, version: Option<&str>) -> InstanceEntry {
        InstanceEntry {
            folder: folder.into(),
            name: name.into(),
            version_id: version.map(str::to_string),
            installation: None,
            min_memory_mb: 512,
            max_memory_mb: 2048,
            jvm_arguments: Vec::new(),
            fullscreen: false,
            width: None,
            height: None,
            override_window: false,
            override_memory: false,
            override_java: false,
            override_jvm_arguments: false,
            java_path: None,
            last_opened_at: None,
            icon_path: None,
        }
    }

    #[test]
    fn resolve_prefers_an_exact_folder() {
        let entries = [
            InstanceRef {
                folder: "Survival".into(),
                name: "Old Name".into(),
            },
            InstanceRef {
                folder: "other".into(),
                name: "Survival".into(),
            },
        ];

        let resolved = resolve_instance("Survival", &entries).expect("folder match");
        assert_eq!(resolved.folder, "Survival");
    }

    #[test]
    fn resolve_accepts_a_unique_display_name() {
        let entries = [InstanceRef {
            folder: "survival-1".into(),
            name: "Survival".into(),
        }];

        let resolved = resolve_instance("Survival", &entries).expect("name match");
        assert_eq!(resolved.folder, "survival-1");
    }

    #[test]
    fn resolve_rejects_an_ambiguous_display_name() {
        let entries = [
            InstanceRef {
                folder: "survival-1".into(),
                name: "Survival".into(),
            },
            InstanceRef {
                folder: "survival-2".into(),
                name: "Survival".into(),
            },
        ];

        assert_eq!(
            resolve_instance("Survival", &entries),
            Err(InstanceResolveError::Ambiguous {
                folders: vec!["survival-1".into(), "survival-2".into()],
            })
        );
    }

    #[test]
    fn resolve_rejects_a_missing_instance() {
        assert_eq!(
            resolve_instance("Missing", &[]),
            Err(InstanceResolveError::Missing)
        );
    }

    #[test]
    fn non_interactive_list_uses_tabs_and_a_dash_for_missing_versions() {
        let entries = [entry("survival", "Survival", None)];

        assert_eq!(
            format_instances_list(&entries, auto(false)),
            vec!["survival\tSurvival\t-".to_string()]
        );
    }

    #[test]
    fn interactive_list_prints_an_aligned_table_with_headers() {
        let entries = [
            entry(
                "Cobblemon-Official-Modpack-Fabric",
                "Cobblemon Official Modpack [Fabric]",
                Some("fabric-loader-0.20.1-1.21.1"),
            ),
            entry(
                "Fabulously-Optimized",
                "Fabulously Optimized",
                Some("fabric-loader-0.20.1-26.3"),
            ),
        ];

        assert_eq!(
            format_instances_list(&entries, auto(true)),
            vec![
                "FOLDER                             NAME                                 VERSION".to_string(),
                "─────────────────────────────────  ───────────────────────────────────  ───────────────────────────".to_string(),
                "Cobblemon-Official-Modpack-Fabric  Cobblemon Official Modpack [Fabric]  fabric-loader-0.20.1-1.21.1".to_string(),
                "Fabulously-Optimized               Fabulously Optimized                 fabric-loader-0.20.1-26.3".to_string(),
            ]
        );
    }

    #[test]
    fn json_list_prints_stable_objects() {
        let entries = [
            entry("survival", "Survival", Some("1.20.1")),
            entry("empty", "Empty", None),
        ];

        assert_eq!(
            format_instances_list(
                &entries,
                InstancesListOutput {
                    json: true,
                    interactive: true,
                }
            ),
            vec![
                "[\n  {\n    \"folder\": \"survival\",\n    \"name\": \"Survival\",\n    \"versionId\": \"1.20.1\"\n  },\n  {\n    \"folder\": \"empty\",\n    \"name\": \"Empty\",\n    \"versionId\": null\n  }\n]"
                    .to_string()
            ]
        );
    }

    #[test]
    fn json_list_escapes_quotes_in_names() {
        let entries = [entry("pack", "Name \"quoted\"", Some("1.0"))];

        assert_eq!(
            format_instances_list(
                &entries,
                InstancesListOutput {
                    json: true,
                    interactive: false,
                }
            ),
            vec![
                "[\n  {\n    \"folder\": \"pack\",\n    \"name\": \"Name \\\"quoted\\\"\",\n    \"versionId\": \"1.0\"\n  }\n]"
                    .to_string()
            ]
        );
    }

    #[test]
    fn interactive_list_is_empty_when_there_are_no_instances() {
        assert!(format_instances_list(&[], auto(true)).is_empty());
    }
}
