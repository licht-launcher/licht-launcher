use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{CoreError, Result, SharedCache};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AssetIndexFile {
    pub objects: BTreeMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    pub virtual_assets: bool,
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub fn parse_asset_index(json: &str) -> Result<AssetIndexFile> {
    serde_json::from_str(json).map_err(CoreError::AssetIndex)
}

/// Copies hashed objects to the names an old client opens.
///
/// `map_to_resources` lands in `<game directory>/resources`. `virtual` lands in
/// the shared `assets/virtual/<index id>` folder. When both are set, both copies
/// are made and the virtual folder is the one passed as `${game_assets}`.
/// A file that already has the indexed size is left in place.
/// Returns `None` when the index asks for neither copy.
pub fn reconstruct_assets(
    cache: &SharedCache,
    index: &AssetIndexFile,
    index_id: &str,
    game_directory: &Path,
) -> Result<Option<PathBuf>> {
    let mut assets_dir = None;
    if index.map_to_resources {
        let resources = game_directory.join("resources");
        place_objects(cache, index, &resources)?;
        assets_dir = Some(resources);
    }
    if index.virtual_assets {
        let virtual_dir = cache.virtual_assets_dir(index_id)?;
        place_objects(cache, index, &virtual_dir)?;
        assets_dir = Some(virtual_dir);
    }
    Ok(assets_dir)
}

fn place_objects(cache: &SharedCache, index: &AssetIndexFile, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for (logical, object) in &index.objects {
        let target = logical_path(destination, logical)?;
        if let Ok(metadata) = std::fs::metadata(&target)
            && metadata.is_file()
            && metadata.len() == object.size
        {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let source = cache.asset_object(&object.hash)?;
        std::fs::copy(source, &target)?;
    }
    Ok(())
}

/// A logical asset path is slash-separated and stays inside `root`.
fn logical_path(root: &Path, logical: &str) -> Result<PathBuf> {
    if logical.is_empty() {
        return Err(CoreError::CachePath);
    }
    let mut path = root.to_path_buf();
    for part in logical.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.contains('\\')
            || part.contains(':')
            || Path::new(part).is_absolute()
        {
            return Err(CoreError::CachePath);
        }
        path.push(part);
    }
    Ok(path)
}

pub async fn fetch_asset_index(client: &reqwest::Client, url: &str) -> Result<AssetIndexFile> {
    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    parse_asset_index(&body)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{AssetIndexFile, AssetObject, reconstruct_assets};
    use crate::{CoreError, SharedCache};

    const HASH: &str = "abababababababababababababababababababab";

    #[test]
    fn map_to_resources_copies_the_logical_name_and_skips_the_same_size() {
        let (cache, game) = dirs("resources");
        write_object(&cache, b"click");
        let index = index(true, false, b"click".len() as u64);
        let assets = reconstruct_assets(&cache, &index, "pre-1.6", &game)
            .expect("copy")
            .expect("resources dir");
        let sound = game
            .join("resources")
            .join("newsound")
            .join("random")
            .join("click.ogg");
        assert_eq!(assets, game.join("resources"));
        assert_eq!(std::fs::read(&sound).expect("sound"), b"click");

        std::fs::write(&sound, b"other").expect("same size, different bytes");
        write_object(&cache, b"click");
        reconstruct_assets(&cache, &index, "pre-1.6", &game).expect("second copy");
        assert_eq!(std::fs::read(&sound).expect("kept"), b"other");
    }

    #[test]
    fn virtual_assets_go_to_the_shared_cache() {
        let (cache, game) = dirs("virtual");
        write_object(&cache, b"music");
        let index = index(false, true, b"music".len() as u64);
        let assets = reconstruct_assets(&cache, &index, "legacy", &game)
            .expect("copy")
            .expect("virtual dir");
        let sound = assets.join("newsound").join("random").join("click.ogg");
        assert_eq!(assets, cache.virtual_assets_dir("legacy").expect("virtual"));
        assert_eq!(std::fs::read(sound).expect("sound"), b"music");
        assert!(!game.join("resources").exists());
    }

    #[test]
    fn both_flags_copy_twice_and_the_argument_is_the_virtual_folder() {
        let (cache, game) = dirs("both");
        write_object(&cache, b"both!");
        let index = index(true, true, b"both!".len() as u64);
        let assets = reconstruct_assets(&cache, &index, "legacy", &game)
            .expect("copy")
            .expect("virtual dir");
        assert_eq!(assets, cache.virtual_assets_dir("legacy").expect("virtual"));
        assert_eq!(
            std::fs::read(game.join("resources/newsound/random/click.ogg")).expect("resources"),
            b"both!"
        );
        assert_eq!(
            std::fs::read(assets.join("newsound/random/click.ogg")).expect("virtual"),
            b"both!"
        );
    }

    #[test]
    fn a_modern_index_copies_nothing() {
        let (cache, game) = dirs("modern");
        write_object(&cache, b"click");
        let index = index(false, false, b"click".len() as u64);
        let assets = reconstruct_assets(&cache, &index, "1.8", &game).expect("no copy");
        assert!(assets.is_none());
        assert!(!game.join("resources").exists());
        assert!(
            cache
                .virtual_assets_dir("1.8")
                .expect("path")
                .starts_with(cache.root())
        );
        assert!(!cache.root().join("assets").join("virtual").exists());
    }

    #[test]
    fn a_path_that_escapes_the_folder_is_rejected() {
        let (cache, game) = dirs("escape");
        write_object(&cache, b"click");
        for logical in [
            "",
            ".",
            "..",
            "../secret",
            "foo/../secret",
            "foo/./bar",
            "foo\\bar",
        ] {
            let mut index = index(true, false, b"click".len() as u64);
            index.objects.clear();
            index.objects.insert(
                logical.to_string(),
                AssetObject {
                    hash: HASH.to_string(),
                    size: b"click".len() as u64,
                },
            );
            let error = reconstruct_assets(&cache, &index, "pre-1.6", &game).expect_err(logical);
            assert!(matches!(error, CoreError::CachePath), "{logical}");
        }
        assert!(!game.join("secret").exists());
        assert!(!game.join("resources").join("secret").exists());
    }

    #[test]
    fn a_missing_object_fails() {
        let (cache, game) = dirs("missing");
        let error = reconstruct_assets(&cache, &index(true, false, 5), "pre-1.6", &game)
            .expect_err("missing object");
        assert!(matches!(error, CoreError::Io(_)));
    }

    fn dirs(name: &str) -> (SharedCache, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("licht-assets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cache = SharedCache::at(root.join("data"));
        let game = root.join("game");
        std::fs::create_dir_all(&game).expect("game dir");
        (cache, game)
    }

    fn write_object(cache: &SharedCache, bytes: &[u8]) {
        let path = cache.asset_object(HASH).expect("object path");
        std::fs::create_dir_all(path.parent().expect("prefix")).expect("objects");
        std::fs::write(path, bytes).expect("object");
    }

    fn index(map_to_resources: bool, virtual_assets: bool, size: u64) -> AssetIndexFile {
        AssetIndexFile {
            objects: BTreeMap::from([(
                "newsound/random/click.ogg".to_string(),
                AssetObject {
                    hash: HASH.to_string(),
                    size,
                },
            )]),
            virtual_assets,
            map_to_resources,
        }
    }
}
