use std::path::{Path, PathBuf};

use crate::{CoreError, Result};

/// Joins `relative` onto `instance_dir` after rejecting anything that could
/// escape it or overwrite launcher files.
///
/// `protected_root_names` are file names that may not appear at the instance
/// root (compared case-insensitively). Nested files with the same name are fine.
pub(super) fn safe_instance_path(
    instance_dir: &Path,
    relative: &str,
    protected_root_names: &[&str],
) -> Result<PathBuf> {
    let normalized = relative.replace('\\', "/");
    // `C:foo` is not absolute on Windows, but joining it replaces the drive.
    // A colon is never a normal mod path component.
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized.contains(':')
    {
        return Err(CoreError::ModpackArchive);
    }
    let mut destination = instance_dir.to_path_buf();
    let mut depth = 0usize;
    for part in normalized.split('/') {
        if !path_component_allowed(part) {
            return Err(CoreError::ModpackArchive);
        }
        if depth == 0
            && protected_root_names
                .iter()
                .any(|protected| part.eq_ignore_ascii_case(protected))
        {
            return Err(CoreError::ModpackArchive);
        }
        destination.push(part);
        depth += 1;
    }
    if depth == 0 {
        return Err(CoreError::ModpackArchive);
    }
    Ok(destination)
}

pub(super) fn path_component_allowed(part: &str) -> bool {
    if part.is_empty() || part == "." || part == ".." {
        return false;
    }
    // Windows strips trailing dots and spaces, so `instance.json.` can alias the profile.
    if part.ends_with('.') || part.ends_with(' ') {
        return false;
    }
    !is_windows_reserved(part)
}

/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, including `CON.txt`.
pub(super) fn is_windows_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part);
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ((upper.starts_with("COM") || upper.starts_with("LPT"))
        && upper.len() == 4
        && upper.as_bytes()[3].is_ascii_digit()
        && upper.as_bytes()[3] != b'0')
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROTECTED: &[&str] = &["instance.json", "licht-modpack.json"];

    #[test]
    fn rejects_protected_files_reserved_names_and_drive_prefixes() {
        for path in [
            "instance.json",
            "Instance.JSON",
            "licht-modpack.json",
            "C:evil.jar",
            "mods/C:evil.jar",
            "CON",
            "mods/CON.txt",
            "mods/lpt1.jar",
            "mods/foo.",
            "../evil.jar",
            "/etc/passwd",
            "mods/../../outside.jar",
            "",
            "mods//a.jar",
            "mods/a\0.jar",
        ] {
            let error =
                safe_instance_path(Path::new("/tmp/instance"), path, PROTECTED).expect_err(path);
            assert!(matches!(error, CoreError::ModpackArchive), "{path}");
        }
    }

    #[test]
    fn protected_names_only_apply_at_the_root() {
        let path = safe_instance_path(
            Path::new("/tmp/instance"),
            "config/instance.json",
            PROTECTED,
        )
        .expect("a nested instance.json is a normal config file");
        assert!(
            path.to_string_lossy()
                .replace('\\', "/")
                .ends_with("config/instance.json")
        );
    }

    #[test]
    fn protected_names_come_from_the_caller() {
        assert!(safe_instance_path(Path::new("/tmp/instance"), "instance.json", &[]).is_ok());
        assert!(safe_instance_path(Path::new("/tmp/instance"), "a.txt", &["A.TXT"]).is_err());
    }

    #[test]
    fn accepts_backslash_separators() {
        let path =
            safe_instance_path(Path::new("/tmp/instance"), "mods\\a.jar", PROTECTED).expect("path");
        assert!(
            path.to_string_lossy()
                .replace('\\', "/")
                .ends_with("mods/a.jar")
        );
    }

    #[test]
    fn detects_windows_reserved_names() {
        for name in ["CON", "con.txt", "NUL", "COM1", "lpt9.jar", "CONIN$"] {
            assert!(is_windows_reserved(name), "{name}");
        }
        for name in ["COM0", "COM10", "console.txt", "a.jar"] {
            assert!(!is_windows_reserved(name), "{name}");
        }
    }

    #[test]
    fn rejects_dot_segments_and_trailing_dots_or_spaces() {
        for part in ["", ".", "..", "a.", "a "] {
            assert!(!path_component_allowed(part), "{part:?}");
        }
        assert!(path_component_allowed("a.jar"));
    }
}
