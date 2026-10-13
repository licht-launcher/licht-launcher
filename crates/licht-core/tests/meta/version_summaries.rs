use licht_core::{
    SharedCache, VersionType, is_version_ready, parse_version_manifest, version_summaries,
};

fn manifest() -> licht_core::VersionManifest {
    parse_version_manifest(
        r#"{
            "latest": {"release": "1.2", "snapshot": "s"},
            "versions": [
                {
                    "id": "1.2",
                    "type": "release",
                    "url": "https://example.invalid/1.2.json",
                    "time": "t",
                    "releaseTime": "t",
                    "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "complianceLevel": 0
                },
                {
                    "id": "bad/id",
                    "type": "snapshot",
                    "url": "https://example.invalid/bad.json",
                    "time": "t",
                    "releaseTime": "t",
                    "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "complianceLevel": 0
                }
            ]
        }"#,
    )
    .expect("manifest")
}

#[test]
fn a_saved_version_json_is_installed_and_a_bad_id_is_not() {
    let root = std::env::temp_dir().join(format!("licht-summaries-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let json = cache.version_json("1.2").expect("version path");
    std::fs::create_dir_all(json.parent().expect("parent")).expect("version dir");
    std::fs::write(&json, "{}").expect("version json");

    let summaries = version_summaries(&manifest(), &cache);
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, "1.2");
    assert!(summaries[0].installed);
    assert_eq!(summaries[1].id, "bad/id");
    assert!(!summaries[1].installed);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_local_loader_profile_is_listed_and_needs_its_jar() {
    let root = std::env::temp_dir().join(format!(
        "licht-summaries-loader-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let cache = SharedCache::at(&root);
    let id = "fabric-loader-0.16.0-1.20.1";
    let json = cache.version_json(id).expect("version path");
    std::fs::create_dir_all(json.parent().expect("parent")).expect("version dir");
    std::fs::write(&json, r#"{"inheritsFrom":"1.20.1"}"#).expect("version json");

    let summaries = version_summaries(&manifest(), &cache);
    let loader = summaries
        .iter()
        .find(|entry| entry.id == id)
        .expect("local loader summary");
    assert_eq!(loader.version_type, VersionType::OldAlpha);
    assert!(!loader.installed);

    let jar = cache.version_jar(id).expect("jar path");
    std::fs::write(&jar, b"client").expect("jar");
    let summaries = version_summaries(&manifest(), &cache);
    let loader = summaries
        .iter()
        .find(|entry| entry.id == id)
        .expect("local loader summary");
    assert!(loader.installed);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn provisional_forge_profiles_are_not_launch_ready() {
    let root = std::env::temp_dir().join(format!("licht-forge-ready-{}", std::process::id()));
    let cache = SharedCache::at(&root);
    let parent = cache.version_json("1.20.1").expect("parent path");
    std::fs::create_dir_all(parent.parent().expect("parent directory")).expect("mkdir");
    std::fs::write(parent, "{}").expect("parent metadata");
    std::fs::write(cache.version_jar("1.20.1").expect("jar"), b"client").expect("client");
    for family in ["forge", "neoforge"] {
        let build = if family == "forge" {
            "1.20.1-47.3.0"
        } else {
            "20.1.80"
        };
        let id = format!("{family}-{build}");
        let path = cache.version_json(&id).expect("profile path");
        std::fs::create_dir_all(path.parent().expect("profile directory")).expect("mkdir");
        let profile = serde_json::json!({
            "id": id, "inheritsFrom": "1.20.1", "mainClass": "net.minecraft.client.main.Main",
            "libraries": [], "lichtLoader": {"family": family, "build": build, "gameVersion": "1.20.1"}
        });
        std::fs::write(path, profile.to_string()).expect("provisional profile");
        assert!(
            !is_version_ready(&cache, &id),
            "a vanilla parent is not a completed loader"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}
