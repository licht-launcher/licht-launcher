use std::path::PathBuf;

use clap::{Args, ValueEnum};
use licht_core::{
    CoreError, SharedCache, VersionManifest, VersionType, fetch_version_manifest, game_cache,
    version_type_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VersionTypeFilter {
    Release,
    Snapshot,
    #[value(name = "old_beta")]
    OldBeta,
    #[value(name = "old_alpha")]
    OldAlpha,
}

impl From<VersionTypeFilter> for VersionType {
    fn from(value: VersionTypeFilter) -> Self {
        match value {
            VersionTypeFilter::Release => VersionType::Release,
            VersionTypeFilter::Snapshot => VersionType::Snapshot,
            VersionTypeFilter::OldBeta => VersionType::OldBeta,
            VersionTypeFilter::OldAlpha => VersionType::OldAlpha,
        }
    }
}

#[derive(Debug, Args)]
pub struct VersionsArgs {
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
    /// Only list versions of this type.
    #[arg(long = "type", value_name = "TYPE", value_enum)]
    pub version_type: Option<VersionTypeFilter>,
    /// Print versions as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone)]
pub struct VersionsListOutput {
    pub version_type: Option<VersionType>,
    pub json: bool,
}

pub async fn versions(args: VersionsArgs) -> licht_core::Result<i32> {
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&licht_core::initialize_launcher().await?).await?,
    };
    let client = reqwest::Client::builder()
        .timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let manifest = fetch_version_manifest(&client, &cache).await?;
    for line in format_versions_list(
        &manifest,
        VersionsListOutput {
            version_type: args.version_type.map(VersionType::from),
            json: args.json,
        },
    ) {
        println!("{line}");
    }
    Ok(0)
}

pub fn format_versions_list(manifest: &VersionManifest, output: VersionsListOutput) -> Vec<String> {
    let versions = manifest
        .versions
        .iter()
        .filter(|version| match &output.version_type {
            Some(wanted) => &version.version_type == wanted,
            None => true,
        })
        .collect::<Vec<_>>();

    if output.json {
        vec![format_versions_json(&versions)]
    } else {
        versions
            .iter()
            .map(|version| {
                format!(
                    "{} {}",
                    version.id,
                    version_type_name(&version.version_type)
                )
            })
            .collect()
    }
}

fn format_versions_json(versions: &[&licht_core::ManifestVersion]) -> String {
    if versions.is_empty() {
        return "[]".to_string();
    }

    let mut body = String::from("[\n");
    for (index, version) in versions.iter().enumerate() {
        if index > 0 {
            body.push_str(",\n");
        }
        body.push_str("  {\n");
        body.push_str(&format!("    \"id\": {},\n", json_string(&version.id)));
        body.push_str(&format!(
            "    \"type\": {}\n",
            json_string(version_type_name(&version.version_type))
        ));
        body.push_str("  }");
    }
    body.push_str("\n]");
    body
}

fn json_string(value: &str) -> String {
    let mut encoded = String::from('"');
    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{08}' => encoded.push_str("\\b"),
            '\u{0c}' => encoded.push_str("\\f"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            character if character.is_control() => {
                encoded.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => encoded.push(character),
        }
    }
    encoded.push('"');
    encoded
}

#[cfg(test)]
mod tests {
    use super::{VersionsListOutput, format_versions_list};
    use licht_core::{LatestVersions, ManifestVersion, VersionManifest, VersionType};

    fn manifest() -> VersionManifest {
        VersionManifest {
            latest: LatestVersions {
                release: "1.20.1".into(),
                snapshot: "23w31a".into(),
            },
            versions: vec![
                ManifestVersion {
                    id: "23w31a".into(),
                    version_type: VersionType::Snapshot,
                    url: "https://example.test/23w31a".into(),
                    time: "2023-01-01T00:00:00+00:00".into(),
                    release_time: "2023-01-01T00:00:00+00:00".into(),
                    sha1: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    compliance_level: 1,
                },
                ManifestVersion {
                    id: "1.20.1".into(),
                    version_type: VersionType::Release,
                    url: "https://example.test/1.20.1".into(),
                    time: "2023-01-02T00:00:00+00:00".into(),
                    release_time: "2023-01-02T00:00:00+00:00".into(),
                    sha1: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
                    compliance_level: 1,
                },
            ],
        }
    }

    #[test]
    fn lists_id_and_type_by_default() {
        assert_eq!(
            format_versions_list(
                &manifest(),
                VersionsListOutput {
                    version_type: None,
                    json: false,
                }
            ),
            vec!["23w31a snapshot".to_string(), "1.20.1 release".to_string()]
        );
    }

    #[test]
    fn filters_by_version_type() {
        assert_eq!(
            format_versions_list(
                &manifest(),
                VersionsListOutput {
                    version_type: Some(VersionType::Release),
                    json: false,
                }
            ),
            vec!["1.20.1 release".to_string()]
        );
    }

    #[test]
    fn prints_stable_json() {
        assert_eq!(
            format_versions_list(
                &manifest(),
                VersionsListOutput {
                    version_type: Some(VersionType::Snapshot),
                    json: true,
                }
            ),
            vec!["[\n  {\n    \"id\": \"23w31a\",\n    \"type\": \"snapshot\"\n  }\n]".to_string()]
        );
    }
}
