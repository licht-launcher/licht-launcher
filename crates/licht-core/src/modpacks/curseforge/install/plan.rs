use std::collections::{HashMap, HashSet};
use std::path::Path;

use reqwest::Client;

use super::PROTECTED_ROOT_NAMES;
use super::manifest::ManifestFile;
use crate::modpacks::curseforge::api::{RemoteFile, RemoteMod, resolve_remote};
use crate::modpacks::curseforge::cdn::canonical_download_url;
use crate::modpacks::path_safety::{path_component_allowed, safe_instance_path};
use crate::{CoreError, Result};

#[derive(Debug, Default)]
pub(super) struct Plan {
    pub(super) files: Vec<PlannedFile>,
    pub(super) blocked: Vec<BlockedMod>,
}

#[derive(Debug)]
pub(super) struct PlannedFile {
    pub(super) path: String,
    pub(super) url: String,
    pub(super) sha1: Option<String>,
    pub(super) md5: Option<String>,
    pub(super) size: u64,
    pub(super) project_id: u64,
    pub(super) file_id: u64,
    pub(super) file_name: String,
    pub(super) name: String,
    pub(super) required: bool,
}

#[derive(Debug)]
pub(super) struct BlockedMod {
    pub(super) name: String,
    pub(super) file_name: String,
}

pub(super) async fn resolve_plan(
    client: &Client,
    key: &str,
    manifest_files: &[ManifestFile],
) -> Result<Plan> {
    let file_ids = manifest_files.iter().map(|file| file.file_id).collect();
    let (remote, mods) = resolve_remote(client, key, file_ids).await?;
    plan_files(manifest_files, &remote, &mods)
}

fn plan_files(
    manifest_files: &[ManifestFile],
    remote_files: &[RemoteFile],
    mods: &[RemoteMod],
) -> Result<Plan> {
    let mut by_file: HashMap<u64, &RemoteFile> = HashMap::new();
    for file in remote_files {
        by_file.entry(file.file_id).or_insert(file);
    }
    let mut by_mod: HashMap<u64, &RemoteMod> = HashMap::new();
    for project in mods {
        by_mod.entry(project.id).or_insert(project);
    }
    let mut plan = Plan::default();
    let mut seen_files = HashSet::new();
    let mut seen_paths = HashSet::new();
    for entry in manifest_files {
        if !seen_files.insert(entry.file_id) {
            continue;
        }
        let Some(remote) = by_file.get(&entry.file_id) else {
            return Err(CoreError::CurseforgeFileMissing {
                file_id: entry.file_id,
            });
        };
        let project = by_mod.get(&remote.mod_id).copied();
        let name = mod_label(
            project.map(|item| item.name.as_str()),
            &remote.display_name,
            &remote.file_name,
            entry.project_id,
        );
        let Some(raw_url) = remote.download_url.as_deref() else {
            plan.blocked.push(BlockedMod {
                name,
                file_name: one_line(&remote.file_name),
            });
            continue;
        };
        let file_name = safe_download_name(&remote.file_name)?;
        let url = canonical_download_url(raw_url)?;
        let directory = content_directory(project.and_then(|item| item.class_id));
        let path = format!("{directory}/{file_name}");
        if !seen_paths.insert(path.clone()) {
            return Err(CoreError::ModpackArchive);
        }
        // Reject the joined path here, before anything is written.
        let _ = safe_instance_path(Path::new("."), &path, &PROTECTED_ROOT_NAMES)?;
        plan.files.push(PlannedFile {
            path,
            url,
            sha1: remote.sha1.clone(),
            md5: remote.md5.clone(),
            size: remote.size,
            project_id: entry.project_id,
            file_id: entry.file_id,
            file_name,
            name,
            required: entry.required,
        });
    }
    Ok(plan)
}

/// CurseForge Minecraft class ids. Unknown classes land in `mods`.
/// World zips are stored under `saves/` and are not extracted.
fn content_directory(class_id: Option<u64>) -> &'static str {
    match class_id {
        Some(12) => "resourcepacks",
        Some(17) => "saves",
        Some(6552) => "shaderpacks",
        Some(6945) => "datapacks",
        _ => "mods",
    }
}

fn mod_label(
    project_name: Option<&str>,
    display_name: &str,
    file_name: &str,
    project_id: u64,
) -> String {
    let chosen = if let Some(name) = project_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        name.to_string()
    } else if !display_name.trim().is_empty() {
        display_name.trim().to_string()
    } else if !file_name.trim().is_empty() {
        file_name.trim().to_string()
    } else {
        format!("project {project_id}")
    };
    one_line(&chosen)
}

fn one_line(value: &str) -> String {
    value
        .replace(['\n', '\r', '\t'], " ")
        .chars()
        .take(120)
        .collect()
}

pub(super) fn format_blocked(blocked: &[BlockedMod]) -> String {
    let mut items: Vec<&BlockedMod> = blocked.iter().collect();
    items.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    items
        .iter()
        .map(|item| {
            if item.file_name.is_empty() {
                item.name.clone()
            } else {
                format!("{} ({})", item.name, item.file_name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn safe_download_name(file_name: &str) -> Result<String> {
    let name = file_name.trim();
    if name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || !path_component_allowed(name)
    {
        return Err(CoreError::ModpackArchive);
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::api::remote_mods_from_body;

    #[test]
    fn places_files_by_class_and_strips_api_keys_from_urls() {
        let mods = remote_mods_from_body(
            r#"{"data":[
              {"id":1,"name":"Sodium","classId":6},
              {"id":2,"name":"Faithful","classId":12},
              {"id":3,"name":"Complementary","classId":6552},
              {"id":4,"name":"Data","classId":6945},
              {"id":5,"name":"World","classId":17}
            ]}"#,
        )
        .expect("mods");
        let remote = vec![
            remote_at(
                1,
                1,
                "https://edge.forgecdn.net/files/1/1/sodium.jar?api-key=super-secret",
            ),
            remote_at(2, 2, "https://media.forgecdn.net/files/1/2/faithful.zip"),
            remote_at(3, 3, "https://EDGE.FORGECDN.NET/files/1/3/shader.zip"),
            remote_at(4, 4, "https://mediafilez.forgecdn.net/files/1/4/data.zip"),
            remote_at(5, 5, "https://edge.forgecdn.net:443/files/1/5/world.zip"),
        ];
        let manifest = vec![
            manifest_file(10, 1),
            manifest_file(11, 2),
            manifest_file(12, 3),
            manifest_file(13, 4),
            manifest_file(14, 5),
        ];
        let plan = plan_files(&manifest, &remote, &mods).expect("plan");
        assert!(plan.blocked.is_empty());
        assert_eq!(plan.files[0].path, "mods/1.jar");
        assert_eq!(plan.files[1].path, "resourcepacks/2.jar");
        assert_eq!(plan.files[2].path, "shaderpacks/3.jar");
        assert_eq!(plan.files[3].path, "datapacks/4.jar");
        assert_eq!(plan.files[4].path, "saves/5.jar");
        assert!(!plan.files[0].url.contains("super-secret"));
        assert!(plan.files[0].url.contains("edge.forgecdn.net"));
        assert_eq!(plan.files[0].name, "Sodium");
    }

    #[test]
    fn lists_mods_whose_authors_disabled_distribution() {
        let ok = remote_at(1, 9, "https://edge.forgecdn.net/files/1/1/ok.jar");
        let mut hidden = remote_at(2, 8, "https://edge.forgecdn.net/files/1/2/hidden.jar");
        hidden.download_url = None;
        hidden.file_name = "hidden.jar".into();
        hidden.display_name = "File Display".into();
        let mods = vec![RemoteMod {
            id: 8,
            name: "Secret Mod".into(),
            class_id: Some(6),
        }];
        let plan = plan_files(
            &[manifest_file(1, 1), manifest_file(2, 2)],
            &[ok, hidden],
            &mods,
        )
        .expect("plan");
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.blocked.len(), 1);
        assert_eq!(plan.blocked[0].name, "Secret Mod");
        assert_eq!(format_blocked(&plan.blocked), "Secret Mod (hidden.jar)");
    }

    #[test]
    fn rejects_download_hosts_outside_the_curseforge_cdn() {
        for url in [
            "https://github.com/a/b/releases/download/v/a.jar",
            "https://cdn.modrinth.com/data/a.jar",
            "https://edge.forgecdn.net.evil.example/a.jar",
        ] {
            let mut remote = remote_at(1, 2, "https://edge.forgecdn.net/files/1/2/a.jar");
            remote.download_url = Some(url.to_string());
            let error = plan_files(&[manifest_file(2, 1)], &[remote], &[]).expect_err(url);
            assert!(
                matches!(error, CoreError::CurseforgeDownloadHost),
                "{url}: {error}"
            );
            assert_eq!(error.to_string(), "curseforge download host is not allowed");
        }
    }

    #[test]
    fn reports_a_manifest_file_the_api_did_not_return() {
        let error = plan_files(&[manifest_file(1, 9)], &[], &[]).unwrap_err();
        assert!(matches!(
            error,
            CoreError::CurseforgeFileMissing { file_id: 9 }
        ));
    }

    #[test]
    fn rejects_file_names_that_leave_the_instance() {
        let mut remote = remote_at(1, 2, "https://edge.forgecdn.net/files/1/2/a.jar");
        remote.file_name = "../evil.jar".into();
        let error = plan_files(&[manifest_file(2, 1)], &[remote], &[]).unwrap_err();
        assert!(matches!(error, CoreError::ModpackArchive));
    }

    fn manifest_file(project_id: u64, file_id: u64) -> ManifestFile {
        ManifestFile {
            project_id,
            file_id,
            required: true,
        }
    }

    fn remote_at(file_id: u64, mod_id: u64, url: &str) -> RemoteFile {
        RemoteFile {
            file_id,
            mod_id,
            file_name: format!("{file_id}.jar"),
            display_name: format!("File {file_id}"),
            size: 4,
            sha1: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
            md5: None,
            download_url: Some(url.to_string()),
        }
    }
}
