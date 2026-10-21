use std::path::Path;

use serde_json::Value;

use super::index::index_files;
use crate::{CoreError, DownloadTask, Result};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(crate) struct StoredModpackHeader {
    pub format_version: Option<i64>,
    pub game: Option<String>,
    pub version_id: Option<String>,
    pub name: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(crate) struct StoredModpackFile {
    pub relative_path: String,
    pub sha1: String,
    pub sha512: String,
    pub url: String,
    pub size: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredModpack {
    pub header: StoredModpackHeader,
    pub dependencies: Vec<(String, String)>,
    pub files: Vec<StoredModpackFile>,
}

impl StoredModpack {
    pub fn parse(index: &Value, instance_dir: &Path) -> Result<Self> {
        if !index.get("files").is_some_and(Value::is_array) {
            return Err(CoreError::ModpackArchive);
        }
        let dependencies = index
            .get("dependencies")
            .and_then(Value::as_object)
            .ok_or(CoreError::ModpackArchive)?
            .iter()
            .map(|(name, version)| {
                Ok((
                    name.clone(),
                    version
                        .as_str()
                        .ok_or(CoreError::ModpackArchive)?
                        .to_string(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        if !dependencies
            .iter()
            .any(|(name, version)| name == "minecraft" && !version.is_empty())
        {
            return Err(CoreError::ModpackArchive);
        }
        let files = index_files(index, instance_dir)?
            .into_iter()
            .map(|file| {
                let relative = file
                    .task
                    .destination
                    .strip_prefix(instance_dir)
                    .map_err(|_| CoreError::ModpackArchive)?;
                Ok(StoredModpackFile {
                    relative_path: relative
                        .to_str()
                        .ok_or(CoreError::CachePath)?
                        .replace('\\', "/"),
                    sha1: file.task.sha1,
                    sha512: file.sha512,
                    url: file.task.url,
                    size: i64::try_from(file.task.size).map_err(|_| CoreError::ModpackArchive)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            header: StoredModpackHeader {
                format_version: index.get("formatVersion").and_then(Value::as_i64),
                game: text(index, "game"),
                version_id: text(index, "versionId"),
                name: text(index, "name"),
                summary: text(index, "summary"),
            },
            dependencies,
            files,
        })
    }
}

impl StoredModpackFile {
    pub fn task(&self, instance_dir: &Path) -> Result<DownloadTask> {
        Ok(DownloadTask {
            destination: crate::modpacks::path_safety::safe_instance_path(
                instance_dir,
                &self.relative_path,
                &super::PROTECTED_ROOT_NAMES,
            )?,
            url: self.url.clone(),
            sha1: self.sha1.clone(),
            size: u64::try_from(self.size).map_err(|_| CoreError::ModpackArchive)?,
        })
    }
}

fn text(index: &Value, name: &str) -> Option<String> {
    index.get(name).and_then(Value::as_str).map(str::to_string)
}
