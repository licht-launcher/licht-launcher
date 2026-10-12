//! Resolve version JSON that uses Mojang `inheritsFrom` (Fabric, Forge wrappers, …).

use serde_json::{Map, Value};

use crate::cache::SharedCache;
use crate::{CoreError, Result};

/// Reads `versions/<id>/<id>.json` and merges every `inheritsFrom` parent on disk.
pub fn load_resolved_version_json(cache: &SharedCache, id: &str) -> Result<String> {
    let mut chain = Vec::new();
    let mut current = id.to_string();
    for _ in 0..8 {
        let path = cache.version_json(&current)?;
        if !path.is_file() {
            return Err(CoreError::VersionMissing);
        }
        let body = std::fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&body).map_err(CoreError::Version)?;
        let parent = value
            .get("inheritsFrom")
            .and_then(Value::as_str)
            .map(str::to_string);
        chain.push(value);
        match parent {
            Some(next) => current = next,
            None => break,
        }
    }
    // chain[0] is the leaf (loader); last is the root (vanilla).
    let Some(mut merged) = chain.pop() else {
        return Err(CoreError::VersionMissing);
    };
    while let Some(child) = chain.pop() {
        merged = merge_version_json(merged, child);
    }
    Ok(merged.to_string())
}

/// Returns `inheritsFrom` when the local profile JSON declares a parent.
pub fn local_inherits_from(cache: &SharedCache, id: &str) -> Result<Option<String>> {
    let path = cache.version_json(id)?;
    if !path.is_file() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&body).map_err(CoreError::Version)?;
    Ok(value
        .get("inheritsFrom")
        .and_then(Value::as_str)
        .map(str::to_string))
}

pub fn local_version_exists(cache: &SharedCache, id: &str) -> bool {
    cache.version_json(id).is_ok_and(|path| path.is_file())
}

fn merge_version_json(mut parent: Value, child: Value) -> Value {
    let Some(parent_obj) = parent.as_object_mut() else {
        return child;
    };
    let Some(child_obj) = child.as_object() else {
        return parent;
    };

    for key in [
        "id",
        "mainClass",
        "minecraftArguments",
        "type",
        "assets",
        "minimumLauncherVersion",
        "releaseTime",
        "time",
        "complianceLevel",
        "javaVersion",
        "assetIndex",
        "downloads",
        "logging",
    ] {
        if let Some(value) = child_obj.get(key) {
            parent_obj.insert(key.to_string(), value.clone());
        }
    }

    if let (Some(parent_args), Some(child_args)) = (
        parent_obj.get("arguments").and_then(Value::as_object),
        child_obj.get("arguments").and_then(Value::as_object),
    ) {
        let mut merged_args = parent_args.clone();
        append_arg_list(&mut merged_args, child_args, "game");
        append_arg_list(&mut merged_args, child_args, "jvm");
        parent_obj.insert("arguments".to_string(), Value::Object(merged_args));
    } else if let Some(args) = child_obj.get("arguments") {
        parent_obj.insert("arguments".to_string(), args.clone());
    }

    match (
        parent_obj.get("libraries").cloned(),
        child_obj.get("libraries").cloned(),
    ) {
        (Some(Value::Array(mut parent_libs)), Some(Value::Array(child_libs))) => {
            parent_libs.extend(child_libs);
            parent_obj.insert("libraries".to_string(), Value::Array(parent_libs));
        }
        (_, Some(libs @ Value::Array(_))) => {
            parent_obj.insert("libraries".to_string(), libs);
        }
        _ => {}
    }

    parent_obj.remove("inheritsFrom");
    parent
}

fn append_arg_list(target: &mut Map<String, Value>, source: &Map<String, Value>, key: &str) {
    let Some(Value::Array(extra)) = source.get(key) else {
        return;
    };
    match target.get_mut(key) {
        Some(Value::Array(existing)) => existing.extend(extra.iter().cloned()),
        _ => {
            target.insert(key.to_string(), Value::Array(extra.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn child_overrides_main_class_and_appends_libraries() {
        let parent = json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "assetIndex": { "id": "5", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example" },
            "downloads": { "client": { "sha1": "b", "size": 1, "url": "https://example/client" } },
            "libraries": [{ "name": "com.mojang:authlib:1" }],
            "arguments": { "game": ["--username"], "jvm": ["-Xmx1G"] }
        });
        let child = json!({
            "id": "fabric-loader-0.16.0-1.20.1",
            "inheritsFrom": "1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [{ "name": "net.fabricmc:fabric-loader:0.16.0" }],
            "arguments": { "game": ["--demo"], "jvm": ["-Dfabric=1"] }
        });
        let merged = merge_version_json(parent, child);
        assert_eq!(
            merged["mainClass"],
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert!(merged.get("inheritsFrom").is_none());
        let libs = merged["libraries"].as_array().expect("libs");
        assert_eq!(libs.len(), 2);
        let game = merged["arguments"]["game"].as_array().expect("game");
        assert_eq!(game.len(), 2);
    }

    #[test]
    fn loads_fabric_profile_merged_with_parent_on_disk() {
        let root = std::env::temp_dir().join(format!(
            "licht-resolve-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let cache = crate::SharedCache::at(&root);

        let parent_dir = root.join("versions").join("1.20.1");
        let child_dir = root.join("versions").join("fabric-loader-0.16.0-1.20.1");
        std::fs::create_dir_all(&parent_dir).expect("parent dir");
        std::fs::create_dir_all(&child_dir).expect("child dir");
        std::fs::write(
            parent_dir.join("1.20.1.json"),
            r#"{
              "id": "1.20.1",
              "mainClass": "net.minecraft.client.main.Main",
              "assetIndex": {
                "id": "5",
                "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size": 1,
                "totalSize": 1,
                "url": "https://example.invalid/index"
              },
              "downloads": {
                "client": {
                  "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                  "size": 1,
                  "url": "https://example.invalid/client"
                }
              },
              "libraries": [{ "name": "com.mojang:authlib:1.0",
                "downloads": { "artifact": {
                  "path": "com/mojang/authlib/1.0/authlib-1.0.jar",
                  "sha1": "cccccccccccccccccccccccccccccccccccccccc",
                  "size": 1,
                  "url": "https://example.invalid/authlib"
                }}}],
              "arguments": { "game": ["--username"], "jvm": ["-Xmx1G"] }
            }"#,
        )
        .expect("parent json");
        std::fs::write(
            child_dir.join("fabric-loader-0.16.0-1.20.1.json"),
            r#"{
              "id": "fabric-loader-0.16.0-1.20.1",
              "inheritsFrom": "1.20.1",
              "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
              "libraries": [
                {
                  "name": "net.fabricmc:fabric-loader:0.16.0",
                  "url": "https://maven.fabricmc.net/"
                }
              ],
              "arguments": { "game": [], "jvm": ["-Dfabric=1"] }
            }"#,
        )
        .expect("child json");

        let resolved =
            load_resolved_version_json(&cache, "fabric-loader-0.16.0-1.20.1").expect("resolve");
        let version = crate::parse_version(&resolved).expect("parse merged");
        assert_eq!(
            version.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert!(!version.asset_index.url.is_empty());
        assert_eq!(version.libraries.len(), 2);
        let fabric = version
            .libraries
            .iter()
            .find(|library| library.name.contains("fabric-loader"))
            .expect("fabric lib");
        assert!(
            fabric
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .is_some()
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
