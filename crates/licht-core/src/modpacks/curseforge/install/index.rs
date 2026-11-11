use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::PROTECTED_ROOT_NAMES;
use super::plan::Plan;
use crate::download::DownloadTask;
use crate::modpacks::curseforge::cdn::select_cdn_url;
use crate::modpacks::curseforge::storage::StoredCurseforgeFile;
use crate::modpacks::file_hashes::prepare_destination_sha1_md5;
use crate::modpacks::path_safety::safe_instance_path;
use crate::{CoreError, Result};

/// Shape of the legacy `licht-curseforge.json` written by v0.2.x builds.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyIndex {
    files: Vec<LegacyFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyFile {
    path: String,
    downloads: Vec<String>,
    #[serde(default)]
    hashes: LegacyHashes,
    file_size: u64,
    project_id: u64,
    file_id: u64,
    file_name: String,
    name: String,
    required: bool,
}

#[derive(Debug, Default, Deserialize)]
struct LegacyHashes {
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    md5: Option<String>,
}

#[derive(Debug)]
pub(super) struct PendingDownload {
    pub(super) task: DownloadTask,
    pub(super) md5: Option<String>,
}

pub(super) fn stored_files(plan: &Plan) -> Result<Vec<StoredCurseforgeFile>> {
    plan.files
        .iter()
        .map(|file| {
            verifiable(StoredCurseforgeFile {
                relative_path: file.path.clone(),
                url: file.url.clone(),
                sha1: file.sha1.clone(),
                md5: file.md5.clone(),
                size: number(file.size)?,
                project_id: number(file.project_id)?,
                file_id: number(file.file_id)?,
                file_name: file.file_name.clone(),
                name: file.name.clone(),
                required: file.required,
            })
        })
        .collect()
}

/// Parses a legacy index, rejecting entries that could not be downloaded safely.
pub(super) fn legacy_files(body: &str) -> Result<Vec<StoredCurseforgeFile>> {
    let index: LegacyIndex = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    index
        .files
        .into_iter()
        .map(|file| {
            safe_instance_path(Path::new("."), &file.path, &PROTECTED_ROOT_NAMES)?;
            verifiable(StoredCurseforgeFile {
                url: select_cdn_url(&file.downloads)?,
                relative_path: file.path,
                sha1: file.hashes.sha1,
                md5: file.hashes.md5,
                size: number(file.file_size)?,
                project_id: number(file.project_id)?,
                file_id: number(file.file_id)?,
                file_name: file.file_name,
                name: file.name,
                required: file.required,
            })
        })
        .collect()
}

/// Drops blank hashes and fails closed when neither SHA1 nor MD5 is left.
fn verifiable(mut file: StoredCurseforgeFile) -> Result<StoredCurseforgeFile> {
    file.sha1 = present_hash(&file.sha1);
    file.md5 = present_hash(&file.md5);
    if file.sha1.is_none() && file.md5.is_none() {
        return Err(CoreError::ModpackArchive);
    }
    Ok(file)
}

fn number(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| CoreError::ModpackArchive)
}

pub(super) fn pending_task(
    file: &StoredCurseforgeFile,
    instance_dir: &Path,
) -> Result<Option<PendingDownload>> {
    let destination = safe_instance_path(instance_dir, &file.relative_path, &PROTECTED_ROOT_NAMES)?;
    let url = select_cdn_url(std::slice::from_ref(&file.url))?;
    let sha1 = present_hash(&file.sha1).unwrap_or_default();
    let md5 = present_hash(&file.md5);
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
            size: u64::try_from(file.size).map_err(|_| CoreError::ModpackArchive)?,
        },
        md5,
    }))
}

fn present_hash(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::install::test_support::temp_dir;

    #[test]
    fn rejects_an_index_url_or_path_outside_the_allowlist() {
        let escape = stored(
            "../evil.jar",
            "https://edge.forgecdn.net/files/1/2/evil.jar",
        );
        let error = pending_task(&escape, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let launcher_file = stored(
            "licht-curseforge.json",
            "https://edge.forgecdn.net/files/1/2/a.jar",
        );
        let error = pending_task(&launcher_file, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        let github = stored(
            "mods/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
        );
        let error = pending_task(&github, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::CurseforgeDownloadHost));

        let keyed = stored(
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
        let mut file = stored("mods/a.jar", "https://edge.forgecdn.net/files/1/2/a.jar");
        file.sha1 = Some("  ".into());
        let error = pending_task(&file, Path::new("/tmp/instance")).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));

        file.md5 = Some("5eb63bbbe01eeed093cb22bb8f5acdc3".into());
        let root = temp_dir("md5-only");
        assert!(pending_task(&file, &root).expect("md5 is enough").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_a_legacy_index_and_drops_blank_hashes() {
        let body = r#"{"source":"curseforge","files":[{
            "path":"mods/a.jar","downloads":["https://edge.forgecdn.net/files/1/2/a.jar"],
            "hashes":{"sha1":"","md5":"5eb63bbbe01eeed093cb22bb8f5acdc3"},"fileSize":11,
            "projectId":1,"fileId":2,"fileName":"a.jar","name":"A","required":false
        }]}"#;
        let files = legacy_files(body).expect("legacy index");
        assert_eq!(
            files,
            [StoredCurseforgeFile {
                sha1: None,
                md5: Some("5eb63bbbe01eeed093cb22bb8f5acdc3".into()),
                size: 11,
                required: false,
                ..stored("mods/a.jar", "https://edge.forgecdn.net/files/1/2/a.jar")
            }]
        );
    }

    #[test]
    fn rejects_legacy_entries_that_could_not_be_downloaded_safely() {
        let entry = |path: &str, url: &str, hashes: &str| {
            format!(
                r#"{{"files":[{{"path":"{path}","downloads":["{url}"],"hashes":{hashes},
                "fileSize":1,"projectId":1,"fileId":2,"fileName":"a.jar","name":"A","required":true}}]}}"#
            )
        };
        let cdn = "https://edge.forgecdn.net/files/1/2/a.jar";
        let sha1 = r#"{"sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;
        for (body, host) in [
            (entry("../evil.jar", cdn, sha1), false),
            (entry("instance.json", cdn, sha1), false),
            (entry("mods/a.jar", cdn, "{}"), false),
            (entry("mods/a.jar", "https://example.com/a.jar", sha1), true),
        ] {
            let error = legacy_files(&body).unwrap_err();
            if host {
                assert!(matches!(error, CoreError::CurseforgeDownloadHost), "{body}");
            } else {
                assert!(matches!(error, CoreError::ModpackArchive), "{body}");
            }
        }
        assert!(matches!(
            legacy_files("not json").unwrap_err(),
            CoreError::LoaderJson(_)
        ));
    }

    fn stored(path: &str, url: &str) -> StoredCurseforgeFile {
        StoredCurseforgeFile {
            relative_path: path.into(),
            url: url.into(),
            sha1: Some("a".repeat(40)),
            md5: None,
            size: 1,
            project_id: 1,
            file_id: 2,
            file_name: "a.jar".into(),
            name: "A".into(),
            required: true,
        }
    }
}
