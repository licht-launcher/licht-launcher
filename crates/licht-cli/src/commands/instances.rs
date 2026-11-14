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
        for entry in &entries {
            println!("{}", instance_line(entry));
        }
        Ok(0)
    }
    .await;
    database.shutdown().await?;
    result
}

pub fn instance_line(entry: &InstanceEntry) -> String {
    format!(
        "{}\t{}\t{}",
        entry.folder,
        entry.name,
        entry.version_id.as_deref().unwrap_or("-")
    )
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
    use super::{InstanceRef, InstanceResolveError, instance_line, resolve_instance};
    use licht_core::InstanceEntry;

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
    fn instance_lines_use_tabs_and_a_dash_for_missing_versions() {
        let entry = InstanceEntry {
            folder: "survival".into(),
            name: "Survival".into(),
            version_id: None,
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
        };

        assert_eq!(instance_line(&entry), "survival\tSurvival\t-");
    }
}
