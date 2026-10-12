use std::path::{Path, PathBuf};

use licht_core::{CoreError, SharedCache};

const HASH: &str = "bdf48ef6b5d0d23bbb02e17d04865216179f510a";

#[test]
fn paths_follow_the_mojang_layout_under_an_explicit_root() {
    let root = Path::new("/licht");
    let cache = SharedCache::at(root);

    assert_eq!(
        cache.asset_object(HASH).expect("hash"),
        root.join("assets").join("objects").join("bd").join(HASH)
    );
    assert_eq!(
        cache
            .asset_object("BDF48EF6B5D0D23BBB02E17D04865216179F510A")
            .expect("uppercase hash"),
        cache.asset_object(HASH).expect("lowercase hash")
    );
    assert_eq!(
        cache.asset_index("29").expect("index"),
        root.join("assets").join("indexes").join("29.json")
    );
    assert_eq!(
        cache
            .library("com/mojang/patchy/1.1/patchy-1.1.jar")
            .expect("library"),
        root.join("libraries")
            .join("com")
            .join("mojang")
            .join("patchy")
            .join("1.1")
            .join("patchy-1.1.jar")
    );
    assert_eq!(
        cache.version_jar("1.21.11").expect("jar"),
        root.join("versions").join("1.21.11").join("1.21.11.jar")
    );
    assert_eq!(
        cache.version_json("1.21.11").expect("json"),
        root.join("versions").join("1.21.11").join("1.21.11.json")
    );
}

#[test]
fn unsafe_path_pieces_are_rejected() {
    let cache = SharedCache::at("/licht");

    assert!(matches!(
        cache.asset_object("short"),
        Err(CoreError::CachePath)
    ));
    assert!(matches!(
        cache.library("com/mojang/../../secret"),
        Err(CoreError::CachePath)
    ));
    assert!(matches!(
        cache.version_jar("../1.21"),
        Err(CoreError::CachePath)
    ));
    assert!(matches!(cache.asset_index(""), Err(CoreError::CachePath)));
    assert!(matches!(
        cache.instance_dir(".."),
        Err(CoreError::CachePath)
    ));
    assert!(matches!(cache.instance_dir(""), Err(CoreError::CachePath)));
}

#[test]
fn an_instance_sits_beside_the_data_directory() {
    let root = std::env::temp_dir().join(format!("licht-instance-{}", std::process::id()));
    let cache = SharedCache::at(root.join("data"));
    assert_eq!(
        cache.instance_dir("1.20.1").expect("instance"),
        root.join("instances").join("1.20.1")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn create_makes_the_shared_folders() {
    let root = std::env::temp_dir().join(format!("licht-cache-{}", std::process::id()));
    let cache = SharedCache::at(&root);
    cache.create().expect("the folders should be created");

    for relative in [
        PathBuf::from("assets").join("indexes"),
        PathBuf::from("assets").join("objects"),
        PathBuf::from("libraries"),
        PathBuf::from("natives"),
        PathBuf::from("runtime"),
        PathBuf::from("versions"),
    ] {
        assert!(root.join(relative).is_dir());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_system_directory_is_absolute_and_outside_the_test() {
    let cache = SharedCache::system().expect("the home directory should resolve");
    assert!(cache.root().is_absolute());
}
