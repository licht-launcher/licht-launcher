use std::fs;
use std::io::Cursor;
use std::path::Path;

use zip::ZipArchive;

use super::PROTECTED_ROOT_NAMES;
use crate::modpacks::path_safety::{path_component_allowed, safe_instance_path};
use crate::{CoreError, Result};

pub(super) fn override_prefixes(overrides_dir: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    if let Some(dir) = safe_overrides_dir(overrides_dir) {
        prefixes.push(dir);
    }
    if !prefixes.iter().any(|item| item == "overrides") {
        prefixes.push("overrides".to_string());
    }
    if !prefixes.iter().any(|item| item == "override") {
        prefixes.push("override".to_string());
    }
    prefixes
}

fn safe_overrides_dir(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('/');
    if !path_component_allowed(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

pub(super) fn list_override_files(
    bytes: &[u8],
    prefixes: &[String],
) -> Result<Vec<(usize, String)>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let name = file.name().replace('\\', "/");
        let Some(relative) = relative_override(&name, prefixes) else {
            continue;
        };
        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }
        if file.enclosed_name().is_none() {
            return Err(CoreError::ModpackArchive);
        }
        let relative = relative.to_string();
        let _ = safe_instance_path(Path::new("."), &relative, &PROTECTED_ROOT_NAMES)?;
        files.push((index, relative));
    }
    Ok(files)
}

fn relative_override<'a>(name: &'a str, prefixes: &[String]) -> Option<&'a str> {
    for prefix in prefixes {
        let with_slash = format!("{prefix}/");
        if let Some(relative) = name.strip_prefix(&with_slash) {
            return Some(relative);
        }
    }
    None
}

pub(super) fn extract_override_files(
    bytes: &[u8],
    instance_dir: &Path,
    files: &[(usize, String)],
) -> Result<()> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| CoreError::ModpackArchive)?;
    for (index, relative) in files {
        let mut file = archive
            .by_index(*index)
            .map_err(|_| CoreError::ModpackArchive)?;
        let destination = safe_instance_path(instance_dir, relative, &PROTECTED_ROOT_NAMES)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&destination)?;
        std::io::copy(&mut file, &mut out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::curseforge::install::test_support::zip_with;

    #[test]
    fn rejects_overrides_that_escape_or_replace_launcher_files() {
        let prefixes = override_prefixes("overrides");
        for name in [
            "overrides/instance.json",
            "overrides/licht-modpack.json",
            "overrides/licht-curseforge.json",
            "overrides/C:evil.txt",
            "overrides/CON",
            "overrides/mods/CON.txt",
            "overrides/../../outside.txt",
        ] {
            let bytes = zip_with(&[(name, b"pwned")]);
            let error = list_override_files(&bytes, &prefixes).expect_err(name);
            assert!(
                matches!(error, CoreError::ModpackArchive),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn lists_files_under_the_declared_and_legacy_prefixes() {
        let bytes = zip_with(&[
            ("manifest.json", b"{}"),
            ("custom/options.txt", b"fov:90"),
            ("override/legacy.txt", b"old"),
            ("custom/config/instance.json", b"nested"),
        ]);
        let prefixes = override_prefixes("custom");
        assert_eq!(prefixes, ["custom", "overrides", "override"]);
        let files = list_override_files(&bytes, &prefixes).expect("list");
        let names: Vec<&str> = files.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(names, ["options.txt", "legacy.txt", "config/instance.json"]);
    }
}
