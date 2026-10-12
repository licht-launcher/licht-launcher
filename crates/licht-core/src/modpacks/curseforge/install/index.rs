use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::PROTECTED_ROOT_NAMES;
use super::plan::Plan;
use crate::download::DownloadTask;
use crate::modpacks::curseforge::CURSEFORGE_INDEX_FILE;
use crate::modpacks::curseforge::cdn::select_cdn_url;
use crate::modpacks::file_hashes::prepare_destination_sha1_md5;
use crate::modpacks::path_safety::safe_instance_path;
use crate::{CoreError, Result};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseforgeIndex {
    pub(super) source: String,
    pub(super) files: Vec<IndexFile>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct IndexFile {
    pub(super) path: String,
    pub(super) downloads: Vec<String>,
    pub(super) hashes: IndexHashes,
    pub(super) file_size: u64,
    pub(super) project_id: u64,
    pub(super) file_id: u64,
    pub(super) file_name: String,
    pub(super) name: String,
    pub(super) required: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct IndexHashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) md5: Option<String>,
}

#[derive(Debug)]
pub(super) struct PendingDownload {
    pub(super) task: DownloadTask,
    pub(super) md5: Option<String>,
}

pub(super) fn write_index(instance_dir: &Path, plan: &Plan) -> Result<()> {
    let index = CurseforgeIndex {
        source: "curseforge".to_string(),
        files: plan
            .files
            .iter()
            .map(|file| IndexFile {
                path: file.path.clone(),
                downloads: vec![file.url.clone()],
                hashes: IndexHashes {
                    sha1: file.sha1.clone(),
                    md5: file.md5.clone(),
                },
                file_size: file.size,
                project_id: file.project_id,
                file_id: file.file_id,
                file_name: file.file_name.clone(),
                name: file.name.clone(),
                required: file.required,
            })
            .collect(),
    };
    let body = serde_json::to_string_pretty(&index).map_err(CoreError::LoaderJson)?;
    fs::write(instance_dir.join(CURSEFORGE_INDEX_FILE), body)?;
    Ok(())
}

pub(super) fn pending_task(
    file: &IndexFile,
    instance_dir: &Path,
) -> Result<Option<PendingDownload>> {
    let destination = safe_instance_path(instance_dir, &file.path, &PROTECTED_ROOT_NAMES)?;
    let url = select_cdn_url(&file.downloads)?;
    let sha1 = present_hash(&file.hashes.sha1);
    let md5 = file
        .hashes
        .md5
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    // Fail closed: never download a CurseForge file that cannot be verified.
    if sha1.is_empty() && md5.is_none() {
        return Err(CoreError::ModpackArchive);
    }
    if !prepare_destination_sha1_md5(&destination, &sha1, md5.as_deref())? {
        return Ok(None);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Some(PendingDownload {
        task: DownloadTask {
            url,
            destination,
            sha1,
            size: file.file_size,
        },
        md5,
    }))
}

fn present_hash(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::install::test_support::temp_dir;

    #[test]
    fn rejects_an_index_url_or_path_outside_the_allowlist() {
        let escape = IndexFile {
            path: "../evil.jar".into(),
            downloads: vec!["https://edge.forgecdn.net/files/1/2/evil.jar".into()],
            hashes: IndexHashes {
                sha1: Some("a".repeat(40)),
                md5: None,
            },
            file_size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "evil.jar".into(),
            name: "Evil".into(),
            required: true,
        };
        let error = pending_task(&escape, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let launcher_file = IndexFile {
            path: "licht-curseforge.json".into(),
            ..index_file("mods/a.jar", "https://edge.forgecdn.net/files/1/2/a.jar")
        };
        let error = pending_task(&launcher_file, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let github = index_file(
            "mods/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
        );
        let error = pending_task(&github, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::CurseforgeDownloadHost));

        let keyed = index_file(
            "mods/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar?api-key=secret",
        );
        let root = temp_dir("pick");
        let task = pending_task(&keyed, &root)
            .expect("task")
            .expect("download");
        assert!(task.task.url.contains("edge.forgecdn.net"));
        assert!(!task.task.url.contains("secret"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_a_file_without_sha1_or_md5() {
        let mut file = index_file("mods/a.jar", "https://edge.forgecdn.net/files/1/2/a.jar");
        file.hashes = IndexHashes::default();
        let error = pending_task(&file, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        file.hashes.md5 = Some("5eb63bbbe01eeed093cb22bb8f5acdc3".into());
        let root = temp_dir("md5-only");
        assert!(pending_task(&file, &root).expect("md5 is enough").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    fn index_file(path: &str, url: &str) -> IndexFile {
        IndexFile {
            path: path.into(),
            downloads: vec![url.into()],
            hashes: IndexHashes {
                sha1: Some("a".repeat(40)),
                md5: None,
            },
            file_size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "a.jar".into(),
            name: "A".into(),
            required: true,
        }
    }
}
