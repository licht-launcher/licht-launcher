//! Which loader and Minecraft version a launch version runs, read back from the
//! profile Licht saved when it installed the loader.

use std::fs;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::LoaderPlatform;
use crate::{CoreError, Result, SharedCache};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstancePlatform {
    pub platform: LoaderPlatform,
    pub game_version: String,
    /// The loader build, such as `0.16.10`; `None` for Vanilla.
    pub loader_version: Option<String>,
}

/// The platform behind `version_id`, or `None` when its profile is not in the cache
/// or was written by a loader Licht does not recognize.
pub fn instance_platform(
    cache: &SharedCache,
    version_id: &str,
) -> Result<Option<InstancePlatform>> {
    let body = match fs::read_to_string(cache.version_json(version_id)?) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let profile: Value = serde_json::from_str(&body).map_err(CoreError::LoaderJson)?;
    Ok(platform_of(version_id, &profile))
}

fn platform_of(version_id: &str, profile: &Value) -> Option<InstancePlatform> {
    let Some(game_version) = profile.get("inheritsFrom").and_then(Value::as_str) else {
        return Some(InstancePlatform {
            platform: LoaderPlatform::Vanilla,
            game_version: version_id.to_string(),
            loader_version: None,
        });
    };
    // Forge and NeoForge profiles carry the request Licht installed them from.
    if let Some(request) = profile.get("lichtLoader") {
        let platform = match request.get("family").and_then(Value::as_str)? {
            "forge" => LoaderPlatform::Forge,
            "neoforge" => LoaderPlatform::NeoForge,
            _ => return None,
        };
        return Some(InstancePlatform {
            platform,
            game_version: game_version.to_string(),
            loader_version: request
                .get("build")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    // Fabric-like ids are `<prefix><loader>-<game version>`.
    [
        ("fabric-loader-", LoaderPlatform::Fabric),
        ("quilt-loader-", LoaderPlatform::Quilt),
    ]
    .into_iter()
    .find_map(|(prefix, platform)| {
        let loader = version_id
            .strip_prefix(prefix)?
            .strip_suffix(game_version)?
            .strip_suffix('-')?;
        Some(InstancePlatform {
            platform,
            game_version: game_version.to_string(),
            loader_version: Some(loader.to_string()),
        })
    })
}

#[cfg(test)]
mod tests {
    use licht_testkit::temp::TempDir;
    use serde_json::json;

    use super::*;

    fn platform(id: &str, profile: Value) -> Option<InstancePlatform> {
        platform_of(id, &profile)
    }

    #[test]
    fn a_version_without_a_parent_is_vanilla() {
        assert_eq!(
            platform("1.21.1", json!({"id": "1.21.1"})),
            Some(InstancePlatform {
                platform: LoaderPlatform::Vanilla,
                game_version: "1.21.1".into(),
                loader_version: None,
            })
        );
    }

    #[test]
    fn reads_fabric_and_quilt_builds_from_the_version_id() {
        assert_eq!(
            platform("fabric-loader-0.19.5-26.3", json!({"inheritsFrom": "26.3"})),
            Some(InstancePlatform {
                platform: LoaderPlatform::Fabric,
                game_version: "26.3".into(),
                loader_version: Some("0.19.5".into()),
            })
        );
        assert_eq!(
            platform(
                "quilt-loader-0.26.4-beta.1-1.20.1",
                json!({"inheritsFrom": "1.20.1"})
            )
            .and_then(|platform| platform.loader_version),
            Some("0.26.4-beta.1".into())
        );
    }

    #[test]
    fn reads_forge_and_neoforge_from_the_saved_install_request() {
        assert_eq!(
            platform(
                "neoforge-21.1.77",
                json!({
                    "inheritsFrom": "1.21.1",
                    "lichtLoader": {"family": "neoforge", "build": "21.1.77", "gameVersion": "1.21.1"}
                })
            ),
            Some(InstancePlatform {
                platform: LoaderPlatform::NeoForge,
                game_version: "1.21.1".into(),
                loader_version: Some("21.1.77".into()),
            })
        );
        assert_eq!(
            platform(
                "1.20.1-forge-47.2.0",
                json!({"inheritsFrom": "1.20.1", "lichtLoader": {"family": "forge", "build": "47.2.0"}})
            )
            .map(|platform| platform.platform),
            Some(LoaderPlatform::Forge)
        );
    }

    #[test]
    fn does_not_guess_an_unknown_loader() {
        assert_eq!(
            platform("optifine-1.20.1", json!({"inheritsFrom": "1.20.1"})),
            None
        );
    }

    #[test]
    fn reads_the_profile_from_the_cache_and_reports_a_missing_one() {
        let temp = TempDir::new("instance-platform");
        let cache = SharedCache::at(temp.path());
        assert_eq!(instance_platform(&cache, "26.3").expect("missing"), None);
        let path = cache
            .version_json("fabric-loader-0.19.5-26.3")
            .expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(&path, r#"{"inheritsFrom":"26.3"}"#).expect("profile");
        assert_eq!(
            instance_platform(&cache, "fabric-loader-0.19.5-26.3")
                .expect("profile")
                .map(|platform| platform.platform),
            Some(LoaderPlatform::Fabric)
        );
        fs::write(&path, "not json").expect("broken profile");
        assert!(instance_platform(&cache, "fabric-loader-0.19.5-26.3").is_err());
    }
}
