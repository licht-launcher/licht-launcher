use licht_core::{
    CoreError, GlobalLaunch, InstancePreparationState, InstanceSource, LoaderChannel,
    LoaderPlatform, ModpackSource, complete_instance_preparation, configure_instance_version,
    create_pending_instance, instance_version_for_launch, list_instances, load_instance,
    update_instance_version,
};

#[test]
fn a_modpack_instance_exists_before_its_version_is_resolved_and_cannot_launch() {
    let root = root("pack");
    let source = InstanceSource::Modpack {
        source: ModpackSource::Modrinth,
        project_id: "project".into(),
        version_id: "pack-version".into(),
        icon_url: None,
    };
    let entry =
        create_pending_instance(&root, "My pack", source.clone(), &template()).expect("create");
    assert!(root.join(&entry.folder).join("instance.json").is_file());
    assert_eq!(entry.version_id, None);
    assert_eq!(
        list_instances(&root).expect("instances"),
        vec![entry.clone()]
    );
    let profile = load_instance(&root.join(&entry.folder)).expect("profile");
    assert_eq!(
        profile.installation.as_ref().expect("installation").source,
        source
    );
    assert!(matches!(
        instance_version_for_launch(&profile),
        Err(CoreError::InstancePreparing)
    ));
    assert!(matches!(
        complete_instance_preparation(&root, &entry.folder),
        Err(CoreError::InstancePreparing)
    ));
    update_instance_version(&root, &entry.folder, "fabric-loader-0.16.10-1.21.1")
        .expect("resolved version");
    let profile = load_instance(&root.join(&entry.folder)).expect("resolved profile");
    assert!(matches!(
        instance_version_for_launch(&profile),
        Err(CoreError::InstancePreparing)
    ));
    assert!(matches!(
        complete_instance_preparation(&root, &entry.folder),
        Err(CoreError::InstancePreparing)
    ));
    configure_instance_version(&root, &entry.folder, "fabric-loader-0.16.10-1.21.1")
        .expect("configured profile");
    assert!(matches!(
        instance_version_for_launch(&load_instance(&root.join(&entry.folder)).expect("configured")),
        Err(CoreError::InstancePreparing)
    ));
    complete_instance_preparation(&root, &entry.folder).expect("complete");
    let profile = load_instance(&root.join(&entry.folder)).expect("ready profile");
    assert_eq!(
        instance_version_for_launch(&profile).expect("launchable"),
        "fabric-loader-0.16.10-1.21.1"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn every_loader_keeps_its_selection_on_disk_before_downloading() {
    let root = root("loaders");
    for platform in [
        LoaderPlatform::Fabric,
        LoaderPlatform::Quilt,
        LoaderPlatform::Forge,
        LoaderPlatform::NeoForge,
    ] {
        let source = InstanceSource::Loader {
            platform,
            game_version: "1.21.1".into(),
            channel: LoaderChannel::Manual,
            loader_version: Some("test-build".into()),
        };
        let entry =
            create_pending_instance(&root, &format!("{platform:?}"), source.clone(), &template())
                .expect("create");
        let profile = load_instance(&root.join(entry.folder)).expect("profile");
        assert_eq!(profile.version_id.as_deref(), Some("1.21.1"));
        let installation = profile.installation.as_ref().expect("selection");
        assert_eq!(installation.state, InstancePreparationState::Preparing);
        assert_eq!(installation.source, source);
        assert!(matches!(
            instance_version_for_launch(&profile),
            Err(CoreError::InstancePreparing)
        ));
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn template() -> GlobalLaunch {
    GlobalLaunch {
        max_memory_mb: 2048,
        jvm_arguments: vec![],
        fullscreen: false,
        width: None,
        height: None,
    }
}

fn root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("licht-pending-{name}-{}", std::process::id()))
}
