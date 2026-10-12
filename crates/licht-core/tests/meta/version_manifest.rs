use licht_core::parse_version_manifest;

#[test]
fn parses_the_saved_version_manifest() {
    let json = include_str!("../fixtures/version_manifest_v2.json");
    let manifest = parse_version_manifest(json).expect("the saved manifest should parse");

    assert!(!manifest.versions.is_empty());
    assert!(
        manifest
            .versions
            .iter()
            .any(|version| version.id == manifest.latest.release),
        "the latest release should be listed"
    );
    assert_eq!(manifest.versions[0].sha1.len(), 40);
}
