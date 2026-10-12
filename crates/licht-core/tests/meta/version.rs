use std::error::Error;

use licht_core::{CoreError, GameArguments, parse_version};

#[test]
fn parses_legacy_version_files() {
    for (json, asset_id) in [
        (include_str!("../fixtures/version-1.8.9.json"), "1.8"),
        (include_str!("../fixtures/version-1.12.2.json"), "1.12"),
    ] {
        let version = parse_version(json).expect("the saved version should parse");
        assert!(matches!(version.arguments, GameArguments::Legacy(_)));
        assert!(!version.libraries.is_empty());
        assert_eq!(version.asset_index.id, asset_id);
        let java = version
            .java_version
            .expect("the saved legacy version includes a Java runtime");
        assert_eq!(java.component, "jre-legacy");
        assert_eq!(java.major_version, 8);
    }
}

#[test]
fn parses_modern_version_files() {
    for json in [
        include_str!("../fixtures/version-1.20.6.json"),
        include_str!("../fixtures/version-1.21.11.json"),
    ] {
        let version = parse_version(json).expect("the saved version should parse");
        assert!(matches!(version.arguments, GameArguments::Modern { .. }));
        assert_eq!(version.downloads.client.sha1.len(), 40);
        let java = version
            .java_version
            .expect("the saved modern version includes a Java runtime");
        assert_eq!(java.major_version, 21);
    }
}

#[test]
fn a_version_without_java_version_leaves_that_field_empty() {
    let version = parse_version(&version_json(r#""minecraftArguments": "--username""#))
        .expect("a legacy version without javaVersion should parse");

    assert!(matches!(version.arguments, GameArguments::Legacy(_)));
    assert!(version.java_version.is_none());
}

#[test]
fn missing_or_duplicate_arguments_are_rejected() {
    let missing = parse_version(&version_json("")).expect_err("arguments are required");
    assert!(matches!(missing, CoreError::VersionArguments));

    let duplicate = parse_version(&version_json(
        r#""minecraftArguments": "--username", "arguments": {"game": [], "jvm": []}"#,
    ))
    .expect_err("only one argument format is allowed");
    assert!(matches!(duplicate, CoreError::VersionArguments));
}

#[test]
fn invalid_version_json_keeps_the_parser_error() {
    let error = parse_version("{").expect_err("truncated JSON should fail");

    assert!(error.source().is_some());
    assert_eq!(error.to_string(), "version JSON is invalid");
}

fn version_json(extra_fields: &str) -> String {
    let separator = if extra_fields.is_empty() { "" } else { "," };
    format!(
        r#"{{
            "libraries": [],
            "assetIndex": {{
                "id": "1.8",
                "sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size": 1,
                "totalSize": 2,
                "url": "https://example.invalid/index.json"
            }},
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {{
                "client": {{
                    "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "size": 3,
                    "url": "https://example.invalid/client.jar"
                }}
            }}{separator}{extra_fields}
        }}"#
    )
}
