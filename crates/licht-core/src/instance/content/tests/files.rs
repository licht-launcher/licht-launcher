use super::*;

#[test]
fn lists_and_toggles_content() {
    let cache = cache("list");
    let root = instance(&cache);
    fs::write(root.join("mods").join("sodium.jar"), b"mod").expect("mod");
    fs::write(
        root.join("resourcepacks").join("faithful.zip.disabled"),
        b"pack",
    )
    .expect("pack");

    let entries = list_instance_content(&cache, "classic").expect("list");
    assert_eq!(entries[0].title, "Sodium");
    assert!(!entries[1].enabled);
    set_instance_content_enabled(
        &cache,
        "classic",
        InstanceContentKind::Mods,
        "sodium.jar",
        false,
    )
    .expect("disable");
    assert!(root.join("mods").join("sodium.jar.disabled").is_file());
}

#[test]
fn lists_only_zip_files_for_shader_packs() {
    let cache = cache("shader-zips");
    let root = instance(&cache);
    let shaders = root.join("shaderpacks");
    fs::write(shaders.join("complementary.zip"), b"shader").expect("zip");
    fs::write(shaders.join("bsl.zip.disabled"), b"shader").expect("disabled zip");
    fs::write(shaders.join("complementary.zip.txt"), b"config").expect("config");
    fs::write(shaders.join("notes.txt"), b"notes").expect("notes");

    let entries = list_instance_content(&cache, "classic").expect("list");
    let names = entries
        .iter()
        .filter(|entry| entry.kind == InstanceContentKind::ShaderPacks)
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["bsl.zip", "complementary.zip"]);
}

#[test]
fn rejects_unsafe_names_and_conflicts() {
    let cache = cache("validation");
    let root = instance(&cache);
    fs::write(root.join("mods").join("test.jar"), b"mod").expect("mod");
    fs::write(root.join("mods").join("test.jar.disabled"), b"mod").expect("disabled");
    assert!(matches!(
        remove_instance_content(
            &cache,
            "classic",
            InstanceContentKind::Mods,
            "../test.jar",
            true
        ),
        Err(CoreError::CachePath)
    ));
    assert!(matches!(
        set_instance_content_enabled(
            &cache,
            "classic",
            InstanceContentKind::Mods,
            "test.jar",
            false
        ),
        Err(CoreError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
    ));
}
