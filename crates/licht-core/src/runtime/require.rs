use crate::Version;

/// The runtime a version asks for, when the JSON includes `javaVersion`.
pub fn required_runtime(version: &Version) -> Option<&crate::JavaVersion> {
    version.java_version.as_ref()
}

/// Major version Eclipse Temurin should provide.
///
/// `javaVersion.majorVersion` wins. Without it, the release id decides:
/// 1.17 is 16, 1.18 through 1.20.4 is 17, 1.20.5 and newer is 21, and every
/// other id is 8.
pub fn required_java_major(version_id: &str, version: &Version) -> u32 {
    if let Some(java) = &version.java_version {
        return java.major_version;
    }
    major_from_release_id(version_id)
}

pub(crate) fn major_from_release_id(version_id: &str) -> u32 {
    let Some(release) = parse_release(version_id) else {
        return 8;
    };
    if release >= (1, 20, 5) {
        return 21;
    }
    if release >= (1, 18, 0) {
        return 17;
    }
    if release >= (1, 17, 0) {
        return 16;
    }
    8
}

fn parse_release(version_id: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version_id.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = match parts.next() {
        Some(part) => part.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::{major_from_release_id, required_java_major};
    use crate::{AssetIndex, Download, GameArguments, JavaVersion, Version, VersionDownloads};

    #[test]
    fn a_release_id_picks_the_java_that_release_needs() {
        assert_eq!(major_from_release_id("1.16.5"), 8);
        assert_eq!(major_from_release_id("1.5.2"), 8);
        assert_eq!(major_from_release_id("24w14a"), 8);
        assert_eq!(major_from_release_id("1.17"), 16);
        assert_eq!(major_from_release_id("1.17.1"), 16);
        assert_eq!(major_from_release_id("1.18"), 17);
        assert_eq!(major_from_release_id("1.19.4"), 17);
        assert_eq!(major_from_release_id("1.20"), 17);
        assert_eq!(major_from_release_id("1.20.4"), 17);
        assert_eq!(major_from_release_id("1.20.5"), 21);
        assert_eq!(major_from_release_id("1.21.11"), 21);
        assert_eq!(major_from_release_id("26.3"), 21);
    }

    #[test]
    fn java_version_in_the_json_wins_over_the_release_id() {
        let version = Version {
            arguments: GameArguments::Legacy(String::new()),
            libraries: Vec::new(),
            asset_index: AssetIndex {
                id: "t".to_string(),
                sha1: "a".repeat(40),
                size: 1,
                total_size: 1,
                url: "https://example.invalid".to_string(),
            },
            main_class: "a.B".to_string(),
            downloads: VersionDownloads {
                client: Download {
                    sha1: "b".repeat(40),
                    size: 1,
                    url: "https://example.invalid".to_string(),
                },
                server: None,
            },
            java_version: Some(JavaVersion {
                component: "java-runtime-epsilon".to_string(),
                major_version: 25,
            }),
        };
        assert_eq!(required_java_major("1.16.5", &version), 25);
    }
}
