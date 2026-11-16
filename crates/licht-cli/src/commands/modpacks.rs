use std::path::PathBuf;
use std::sync::Arc;

use clap::{Args, Subcommand, ValueEnum};
use licht_core::{
    CoreError, DownloadJobOperation, DownloadJobStatus, DownloadManager, InstanceSource,
    ModpackListItem, ModpackSearchFilters, ModpackSource, ModpackVersionItem, SharedCache,
    game_cache, list_modpack_versions, search_modpacks,
};
use tokio::sync::mpsc;

use crate::host::host_environment;
use crate::progress::drain_download_progress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliModpackSource {
    Modrinth,
    Curseforge,
}

impl From<CliModpackSource> for ModpackSource {
    fn from(value: CliModpackSource) -> Self {
        match value {
            CliModpackSource::Modrinth => ModpackSource::Modrinth,
            CliModpackSource::Curseforge => ModpackSource::Curseforge,
        }
    }
}

fn source_name(source: ModpackSource) -> &'static str {
    match source {
        ModpackSource::Modrinth => "modrinth",
        ModpackSource::Curseforge => "curseforge",
    }
}

#[derive(Debug, Subcommand)]
pub enum ModpacksCommand {
    /// Search Modrinth and CurseForge modpacks.
    Search(ModpacksSearchArgs),
    /// List versions for a modpack project.
    Versions(ModpacksVersionsArgs),
    /// Create an instance from a remote modpack and install it.
    Install(ModpacksInstallArgs),
}

#[derive(Debug, Args)]
pub struct ModpacksSearchArgs {
    /// Search query.
    pub query: String,
    /// 1-based results page.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub page: u32,
    /// Limit the search to one source.
    #[arg(long, value_enum)]
    pub source: Option<CliModpackSource>,
    /// Limit the search to a Minecraft version id.
    #[arg(long = "game-version", value_name = "ID")]
    pub game_version: Option<String>,
    /// Print results as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ModpacksVersionsArgs {
    /// Modpack source.
    #[arg(long, value_enum)]
    pub source: CliModpackSource,
    /// Project id on that source.
    #[arg(long = "id", value_name = "PROJECT_ID")]
    pub project_id: String,
    /// Print versions as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ModpacksInstallArgs {
    /// Display name for the new instance.
    #[arg(
        long,
        value_name = "NAME",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub name: String,
    /// Modpack source.
    #[arg(long, value_enum)]
    pub source: CliModpackSource,
    /// Project id on that source.
    #[arg(long = "id", value_name = "PROJECT_ID")]
    pub project_id: String,
    /// Version id on that source.
    #[arg(
        long = "version",
        value_name = "VERSION_ID",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub version_id: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

pub async fn modpacks_search(args: ModpacksSearchArgs) -> licht_core::Result<i32> {
    let client = reqwest::Client::builder()
        .timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let filters = ModpackSearchFilters {
        game_version: args.game_version,
        source: args.source.map(ModpackSource::from),
        ..ModpackSearchFilters::default()
    };
    let page = search_modpacks(&client, &args.query, args.page, &filters).await?;
    for line in format_modpack_search(&page.items, args.json) {
        println!("{line}");
    }
    Ok(0)
}

pub async fn modpacks_versions(args: ModpacksVersionsArgs) -> licht_core::Result<i32> {
    let client = reqwest::Client::builder()
        .timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let versions = list_modpack_versions(&client, args.source.into(), &args.project_id).await?;
    for line in format_modpack_versions(&versions, args.json) {
        println!("{line}");
    }
    Ok(0)
}

pub async fn modpacks_install(args: ModpacksInstallArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let environment = host_environment()?;
    let client = reqwest::Client::builder()
        .connect_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .read_timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let settings_db = pool.clone();
    let database = licht_core::DatabaseService::start(pool).await?;
    database.client().recover_download_jobs(&cache).await?;
    let manager = Arc::new(DownloadManager::new(
        settings_db,
        cache,
        environment,
        client,
        database.client(),
    ));

    let mut events = manager.subscribe();
    let (progress_tx, mut progress_rx) = mpsc::channel(32);
    let progress_task = tokio::spawn(async move {
        while let Ok(snapshot) = events.recv().await {
            let _ = progress_tx.send(snapshot.progress).await;
        }
    });
    let progress_printer = tokio::spawn(async move {
        drain_download_progress(&mut progress_rx).await;
    });

    let result = async {
        let entry = manager
            .create(
                &args.name,
                InstanceSource::Modpack {
                    source: args.source.into(),
                    project_id: args.project_id.clone(),
                    version_id: args.version_id.clone(),
                    icon_url: None,
                },
            )
            .await?;
        let snapshot = manager
            .prepare(&entry.folder, DownloadJobOperation::Install)
            .await?;
        match snapshot.status {
            DownloadJobStatus::Completed => {
                println!(
                    "{}\t{}\t{}",
                    entry.folder,
                    entry.name,
                    entry.version_id.as_deref().unwrap_or("-")
                );
                Ok(0)
            }
            DownloadJobStatus::Failed => Err(CoreError::Io(std::io::Error::other(
                snapshot
                    .error
                    .unwrap_or_else(|| "modpack install failed".into()),
            ))),
            other => Err(CoreError::Io(std::io::Error::other(format!(
                "modpack install ended with status {other:?}"
            )))),
        }
    }
    .await;

    drop(manager);
    let _ = progress_task.await;
    let _ = progress_printer.await;
    database.shutdown().await?;
    result
}

pub fn format_modpack_search(items: &[ModpackListItem], json: bool) -> Vec<String> {
    if json {
        vec![format_modpack_search_json(items)]
    } else {
        items
            .iter()
            .map(|item| {
                format!(
                    "{}\t{}\t{}\t{}",
                    source_name(item.source),
                    item.id,
                    item.name,
                    item.downloads
                )
            })
            .collect()
    }
}

pub fn format_modpack_versions(versions: &[ModpackVersionItem], json: bool) -> Vec<String> {
    if json {
        vec![format_modpack_versions_json(versions)]
    } else {
        versions
            .iter()
            .map(|version| {
                format!(
                    "{}\t{}\t{}",
                    version.id,
                    version.name,
                    version.game_versions.join(",")
                )
            })
            .collect()
    }
}

fn format_modpack_search_json(items: &[ModpackListItem]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let mut body = String::from("[\n");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            body.push_str(",\n");
        }
        body.push_str("  {\n");
        body.push_str(&format!(
            "    \"source\": {},\n",
            json_string(source_name(item.source))
        ));
        body.push_str(&format!("    \"id\": {},\n", json_string(&item.id)));
        body.push_str(&format!("    \"name\": {},\n", json_string(&item.name)));
        body.push_str(&format!("    \"downloads\": {}\n", item.downloads));
        body.push_str("  }");
    }
    body.push_str("\n]");
    body
}

fn format_modpack_versions_json(versions: &[ModpackVersionItem]) -> String {
    if versions.is_empty() {
        return "[]".to_string();
    }
    let mut body = String::from("[\n");
    for (index, version) in versions.iter().enumerate() {
        if index > 0 {
            body.push_str(",\n");
        }
        body.push_str("  {\n");
        body.push_str(&format!("    \"id\": {},\n", json_string(&version.id)));
        body.push_str(&format!("    \"name\": {},\n", json_string(&version.name)));
        body.push_str(&format!(
            "    \"gameVersions\": {}\n",
            json_string_array(&version.game_versions)
        ));
        body.push_str("  }");
    }
    body.push_str("\n]");
    body
}

fn json_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }
    let encoded = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{encoded}]")
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

#[cfg(test)]
mod tests {
    use super::{format_modpack_search, format_modpack_versions};
    use licht_core::{ModpackListItem, ModpackSource, ModpackVersionItem};

    #[test]
    fn search_tsv_lists_source_id_name_and_downloads() {
        let items = [ModpackListItem {
            id: "pack".into(),
            source: ModpackSource::Modrinth,
            name: "Cool Pack".into(),
            description: String::new(),
            downloads: 12,
            icon_url: None,
            author: "author".into(),
            updated_at: None,
            loaders: Vec::new(),
            categories: Vec::new(),
        }];
        assert_eq!(
            format_modpack_search(&items, false),
            vec!["modrinth\tpack\tCool Pack\t12".to_string()]
        );
    }

    #[test]
    fn versions_json_is_empty_for_no_results() {
        let versions: [ModpackVersionItem; 0] = [];
        assert_eq!(
            format_modpack_versions(&versions, true),
            vec!["[]".to_string()]
        );
    }
}
