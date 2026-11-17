use std::io::IsTerminal;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use licht_core::{
    CoreError, InstanceEntry, SharedCache, create_instance, delete_instance, game_cache,
    global_launch, list_instances, load_settings, rename_instance,
};

#[derive(Debug, Subcommand)]
pub enum InstancesCommand {
    /// List saved instances.
    List(InstancesListArgs),
    /// Create a vanilla instance profile (does not download game files).
    Create(InstancesCreateArgs),
    /// Rename a saved instance.
    Rename(InstancesRenameArgs),
    /// Delete a saved instance and its folder.
    Delete(InstancesDeleteArgs),
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

#[derive(Debug, Args)]
pub struct InstancesCreateArgs {
    /// Display name for the new instance.
    #[arg(
        long,
        value_name = "NAME",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub name: String,
    /// Game version id to attach to the profile.
    #[arg(
        long = "version",
        value_name = "ID",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub version_id: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InstancesRenameArgs {
    /// Instance folder or unique display name.
    #[arg(
        long = "instance",
        value_name = "NAME_OR_FOLDER",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub instance: String,
    /// New display name.
    #[arg(
        long,
        value_name = "NAME",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub name: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InstancesDeleteArgs {
    /// Instance folder or unique display name.
    #[arg(
        long = "instance",
        value_name = "NAME_OR_FOLDER",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub instance: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
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

pub async fn instances_create(args: InstancesCreateArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let settings = load_settings(&pool).await?;
    let database = licht_core::DatabaseService::start(pool).await?;
    let result = async {
        let root = cache.instances_dir()?;
        let entry = create_instance(
            &database.client(),
            &root,
            &args.name,
            &args.version_id,
            &global_launch(&settings),
        )
        .await?;
        println!("{}", instance_summary_line(&entry));
        Ok(0)
    }
    .await;
    database.shutdown().await?;
    result
}

pub async fn instances_rename(args: InstancesRenameArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let database = licht_core::DatabaseService::start(pool).await?;
    let result = async {
        let root = cache.instances_dir()?;
        let entries = list_instances(&database.client(), &root).await?;
        let refs = instance_refs(&entries);
        let resolved = resolve_instance(&args.instance, &refs)
            .map_err(|error| resolve_error(&args.instance, error))?;
        let entry =
            rename_instance(&database.client(), &root, &resolved.folder, &args.name).await?;
        println!("{}", instance_summary_line(&entry));
        Ok(0)
    }
    .await;
    database.shutdown().await?;
    result
}

pub async fn instances_delete(args: InstancesDeleteArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let database = licht_core::DatabaseService::start(pool).await?;
    let result = async {
        let root = cache.instances_dir()?;
        let entries = list_instances(&database.client(), &root).await?;
        let refs = instance_refs(&entries);
        let resolved = resolve_instance(&args.instance, &refs)
            .map_err(|error| resolve_error(&args.instance, error))?;
        let folder = resolved.folder.clone();
        delete_instance(&database.client(), &root, &folder).await?;
        println!("deleted\t{folder}");
        Ok(0)
    }
    .await;
    database.shutdown().await?;
    result
}

fn instance_summary_line(entry: &InstanceEntry) -> String {
    format!(
        "{}\t{}\t{}",
        entry.folder,
        entry.name,
        instance_version(entry)
    )
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
