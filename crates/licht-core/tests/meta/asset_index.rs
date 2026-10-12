use std::error::Error;

use licht_core::{CoreError, parse_asset_index};

#[test]
fn parses_saved_asset_indexes() {
    for json in [
        include_str!("../fixtures/asset-index-1.8.json"),
        include_str!("../fixtures/asset-index-1.12.json"),
        include_str!("../fixtures/asset-index-16.json"),
        include_str!("../fixtures/asset-index-29.json"),
    ] {
        let index = parse_asset_index(json).expect("the saved asset index should parse");
        assert!(!index.objects.is_empty());
        let icon = index
            .objects
            .get("icons/icon_16x16.png")
            .expect("the icon should be listed");
        assert_eq!(icon.hash.len(), 40);
    }
}

#[test]
fn saved_indexes_keep_the_known_icon_and_leave_flags_off() {
    let legacy = parse_asset_index(include_str!("../fixtures/asset-index-1.8.json"))
        .expect("the 1.8 index should parse");
    let legacy_icon = legacy
        .objects
        .get("icons/icon_16x16.png")
        .expect("the 1.8 icon should be listed");
    assert_eq!(legacy_icon.hash, "bdf48ef6b5d0d23bbb02e17d04865216179f510a");
    assert_eq!(legacy_icon.size, 3665);
    assert!(!legacy.virtual_assets);
    assert!(!legacy.map_to_resources);

    let modern = parse_asset_index(include_str!("../fixtures/asset-index-29.json"))
        .expect("the 29 index should parse");
    let modern_icon = modern
        .objects
        .get("icons/icon_16x16.png")
        .expect("the 29 icon should be listed");
    assert_eq!(modern_icon.hash, "5ff04807c356f1beed0b86ccf659b44b9983e3fa");
    assert_eq!(modern_icon.size, 781);
    assert!(!modern.virtual_assets);
    assert!(!modern.map_to_resources);
}

#[test]
fn explicit_flags_are_preserved() {
    let index = parse_asset_index(
        r#"{
            "virtual": true,
            "map_to_resources": true,
            "objects": {}
        }"#,
    )
    .expect("an index with both flags should parse");

    assert!(index.virtual_assets);
    assert!(index.map_to_resources);
    assert!(index.objects.is_empty());
}

#[test]
fn invalid_asset_index_json_keeps_the_parser_error() {
    let error = parse_asset_index("{").expect_err("truncated JSON should fail");

    assert!(matches!(error, CoreError::AssetIndex(_)));
    assert!(error.source().is_some());
    assert_eq!(error.to_string(), "asset index JSON is invalid");
}
