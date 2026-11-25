use super::*;
use crate::modpacks::curseforge::storage::StoredCurseforgeFile;
use crate::modpacks::mrpack::storage::{StoredModpack, StoredModpackFile, StoredModpackHeader};

async fn database(cache: &SharedCache) -> crate::DatabaseService {
    crate::DatabaseService::start(crate::Database::open(cache).await.expect("database"))
        .await
        .expect("service")
}

fn modrinth_pack(paths: &[&str]) -> StoredModpack {
    StoredModpack {
        header: StoredModpackHeader {
            format_version: Some(1),
            game: Some("minecraft".to_string()),
            version_id: Some("1.0.0".to_string()),
            name: Some("Pack".to_string()),
            summary: None,
        },
        dependencies: vec![("minecraft".to_string(), "1.21.1".to_string())],
        files: paths
            .iter()
            .map(|path| StoredModpackFile {
                relative_path: (*path).to_string(),
                sha1: "0".repeat(40),
                sha512: "0".repeat(128),
                url: "https://cdn.modrinth.com/data/file".to_string(),
                size: 3,
            })
            .collect(),
    }
}

fn curseforge_file(path: &str) -> StoredCurseforgeFile {
    StoredCurseforgeFile {
        relative_path: path.to_string(),
        url: "https://edge.forgecdn.net/files/1/2/file.jar".to_string(),
        sha1: Some("0".repeat(40)),
        md5: None,
        size: 3,
        project_id: 1,
        file_id: 2,
        file_name: "file.jar".to_string(),
        name: "File".to_string(),
        required: true,
    }
}

async fn listed(cache: &SharedCache, database: &crate::DatabaseClient) -> Vec<(String, bool)> {
    let mut snapshots = Vec::new();
    let entries = list_instance_content_progressively(
        cache,
        database,
        &offline_client(),
        "classic",
        false,
        |entries| {
            snapshots.push(entries.to_vec());
            Ok(())
        },
    )
    .await
    .expect("list");
    assert!(
        snapshots[0]
            .iter()
            .zip(&entries)
            .all(|(first, last)| first.from_modpack == last.from_modpack),
        "the first local snapshot already tells modpack content apart"
    );
    entries
        .into_iter()
        .map(|entry| (entry.name, entry.from_modpack))
        .collect()
}

#[tokio::test]
async fn marks_files_from_a_modrinth_pack_including_disabled_ones() {
    let (_temp, cache) = cache("modpack-modrinth");
    let root = instance(&cache);
    for name in ["sodium.jar.disabled", "extra.jar"] {
        fs::write(root.join("mods").join(name), name).expect("mod");
    }
    fs::write(root.join("resourcepacks").join("pack.zip"), b"pack").expect("pack");
    let service = database(&cache).await;
    let database = service.client();
    database
        .replace_modpack(
            &cache.instances_dir().expect("instances"),
            "classic",
            modrinth_pack(&["mods/sodium.jar", "resourcepacks/pack.zip"]),
        )
        .await
        .expect("store pack");

    let entries = listed(&cache, &database).await;

    assert_eq!(
        entries,
        [
            ("extra.jar".to_string(), false),
            ("sodium.jar".to_string(), true),
            ("pack.zip".to_string(), true),
        ]
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn marks_files_from_a_curseforge_pack() {
    let (_temp, cache) = cache("modpack-curseforge");
    let root = instance(&cache);
    for name in ["jei.jar", "extra.jar"] {
        fs::write(root.join("mods").join(name), name).expect("mod");
    }
    let service = database(&cache).await;
    let database = service.client();
    database
        .replace_curseforge_files(
            &cache.instances_dir().expect("instances"),
            "classic",
            vec![curseforge_file("mods/jei.jar")],
        )
        .await
        .expect("store files");

    let entries = listed(&cache, &database).await;

    assert_eq!(
        entries,
        [
            ("extra.jar".to_string(), false),
            ("jei.jar".to_string(), true)
        ]
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn marks_files_the_pack_copied_from_its_overrides() {
    let (_temp, cache) = cache("modpack-overrides");
    let root = instance(&cache);
    fs::write(root.join("mods").join("sodium.jar"), b"mod").expect("mod");
    fs::write(root.join("resourcepacks").join("bundled.zip"), b"pack").expect("pack");
    let service = database(&cache).await;
    let database = service.client();
    let instances = cache.instances_dir().expect("instances");
    database
        .replace_modpack(&instances, "classic", modrinth_pack(&["mods/sodium.jar"]))
        .await
        .expect("store pack");
    database
        .replace_modpack_override_files(
            &instances,
            "classic",
            vec!["resourcepacks/bundled.zip".to_string()],
        )
        .await
        .expect("store overrides");

    let entries = listed(&cache, &database).await;

    assert_eq!(
        entries,
        [
            ("sodium.jar".to_string(), true),
            ("bundled.zip".to_string(), true),
        ]
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn marks_files_a_pack_ships_disabled_whether_or_not_the_user_enabled_them() {
    let (_temp, cache) = cache("modpack-shipped-disabled");
    let root = instance(&cache);
    for name in [
        "flashback.jar.disabled",
        "replaymod.jar",
        "extra.jar.disabled",
    ] {
        fs::write(root.join("mods").join(name), name).expect("mod");
    }
    fs::write(
        root.join("resourcepacks").join("zombies.zip.disabled"),
        b"pack",
    )
    .expect("pack");
    let service = database(&cache).await;
    let database = service.client();
    let instances = cache.instances_dir().expect("instances");
    database
        .replace_modpack(
            &instances,
            "classic",
            modrinth_pack(&["mods/flashback.jar.disabled", "mods/replaymod.jar.disabled"]),
        )
        .await
        .expect("store pack");
    database
        .replace_modpack_override_files(
            &instances,
            "classic",
            vec!["resourcepacks/zombies.zip.disabled".to_string()],
        )
        .await
        .expect("store overrides");

    let entries = listed(&cache, &database).await;

    assert_eq!(
        entries,
        [
            ("extra.jar".to_string(), false),
            ("flashback.jar".to_string(), true),
            ("replaymod.jar".to_string(), true),
            ("zombies.zip".to_string(), true),
        ]
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn marks_a_shader_folder_the_pack_copied_from_its_overrides() {
    let (_temp, cache) = cache("modpack-shader-folder");
    let root = instance(&cache);
    for name in ["Complementary", "Mine"] {
        let folder = root.join("shaderpacks").join(name).join("shaders");
        fs::create_dir_all(&folder).expect("folder pack");
        fs::write(folder.join("final.fsh"), b"shader").expect("shader file");
    }
    let service = database(&cache).await;
    let database = service.client();
    database
        .replace_modpack_override_files(
            &cache.instances_dir().expect("instances"),
            "classic",
            vec!["shaderpacks/Complementary/shaders/final.fsh".to_string()],
        )
        .await
        .expect("store overrides");

    let entries = listed(&cache, &database).await;

    assert_eq!(
        entries,
        [
            ("Complementary".to_string(), true),
            ("Mine".to_string(), false),
        ]
    );
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn treats_everything_as_extra_without_a_modpack() {
    let (_temp, cache) = cache("modpack-none");
    let root = instance(&cache);
    fs::write(root.join("mods").join("sodium.jar"), b"mod").expect("mod");
    let service = database(&cache).await;
    let database = service.client();

    let entries = listed(&cache, &database).await;

    assert_eq!(entries, [("sodium.jar".to_string(), false)]);
    service.shutdown().await.expect("shutdown");
}
