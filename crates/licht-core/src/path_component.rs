//! Shared rules for a single path segment (instance folders, cache paths, modpacks).

/// `true` when `part` is a safe single path segment (no reserved Windows names, no
/// trailing dots/spaces, not `.` / `..`).
pub(crate) fn path_component_allowed(part: &str) -> bool {
    if part.is_empty() || part == "." || part == ".." {
        return false;
    }
    // Windows strips trailing dots and spaces, so `instance.json.` can alias the profile.
    if part.ends_with('.') || part.ends_with(' ') {
        return false;
    }
    !is_windows_reserved(part)
}

/// Like [`path_component_allowed`], plus rejects `:` and NUL.
///
/// On Windows, joining `C:evil` replaces the path instead of nesting under it.
pub(crate) fn path_component_safe(part: &str) -> bool {
    !part.contains(':') && !part.contains('\0') && path_component_allowed(part)
}

/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, including `CON.txt`.
pub(crate) fn is_windows_reserved(part: &str) -> bool {
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
    fn rejects_dot_segments_trailing_junk_and_drive_prefixes() {
        for part in [
            "", ".", "..", "a.", "a ", "C:evil", "foo:bar", "a\0b", "CON",
        ] {
            assert!(!path_component_safe(part), "{part:?}");
        }
        assert!(path_component_safe("a.jar"));
        assert!(path_component_safe("vanilla-1.20"));
    }
}
