//! A pack's name, version and author as written in its own archive, for packs
//! imported from a file, which have no store page to describe them.

use std::io::Cursor;

use serde_json::Value;
use zip::ZipArchive;

use super::ModpackSource;
use super::archive::read_metadata;
use crate::instance::LinkedModpack;

/// Archive text is shown as-is in the UI, so it is trimmed and capped.
const MAX_DETAIL_CHARS: usize = 200;

/// The pack details its index (`.mrpack`) or manifest (CurseForge) declares, or
/// `None` when the archive names no pack or version.
pub(super) fn archive_details(source: ModpackSource, bytes: &[u8]) -> Option<LinkedModpack> {
    let (file, version_key) = match source {
        ModpackSource::Modrinth => ("modrinth.index.json", "versionId"),
        ModpackSource::Curseforge => ("manifest.json", "version"),
    };
    let mut archive = ZipArchive::new(Cursor::new(bytes)).ok()?;
    let body = read_metadata(archive.by_name(file).ok()?).ok()?;
    let index: Value = serde_json::from_str(&body).ok()?;
    Some(LinkedModpack {
        name: text(&index, "name")?,
        author: text(&index, "author"),
        version_name: text(&index, version_key)?,
        published_at: None,
    })
}

fn text(index: &Value, key: &str) -> Option<String> {
    let value = index.get(key)?.as_str()?.trim();
    (!value.is_empty()).then(|| value.chars().take(MAX_DETAIL_CHARS).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::archive::metadata_zip;

    fn zip(name: &str, json: &str) -> Vec<u8> {
        metadata_zip(name, json, json.len())
    }

    #[test]
    fn reads_the_name_and_version_of_a_modrinth_pack() {
        let bytes = zip(
            "modrinth.index.json",
            r#"{"name":" Fresh & Smooth ","versionId":"2.1.0","files":[]}"#,
        );
        assert_eq!(
            archive_details(ModpackSource::Modrinth, &bytes),
            Some(LinkedModpack {
                name: "Fresh & Smooth".into(),
                author: None,
                version_name: "2.1.0".into(),
                published_at: None,
            })
        );
    }

    #[test]
    fn reads_the_name_version_and_author_of_a_curseforge_pack() {
        let bytes = zip(
            "manifest.json",
            r#"{"name":"All the Mods","version":"1.0","author":"ATM Team","files":[]}"#,
        );
        assert_eq!(
            archive_details(ModpackSource::Curseforge, &bytes),
            Some(LinkedModpack {
                name: "All the Mods".into(),
                author: Some("ATM Team".into()),
                version_name: "1.0".into(),
                published_at: None,
            })
        );
    }

    #[test]
    fn has_no_details_without_a_name_or_version() {
        let unnamed = zip("modrinth.index.json", r#"{"versionId":"1.0"}"#);
        let unversioned = zip("manifest.json", r#"{"name":"Pack","version":" "}"#);
        assert_eq!(archive_details(ModpackSource::Modrinth, &unnamed), None);
        assert_eq!(
            archive_details(ModpackSource::Curseforge, &unversioned),
            None
        );
        assert_eq!(archive_details(ModpackSource::Modrinth, b"not a zip"), None);
    }

    #[test]
    fn caps_long_names() {
        let name = "x".repeat(MAX_DETAIL_CHARS + 50);
        let bytes = zip(
            "modrinth.index.json",
            &format!(r#"{{"name":"{name}","versionId":"1.0"}}"#),
        );
        let details = archive_details(ModpackSource::Modrinth, &bytes).expect("details");
        assert_eq!(details.name.chars().count(), MAX_DETAIL_CHARS);
    }
}
