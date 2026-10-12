use std::collections::BTreeMap;

use licht_core::{
    Arch, AssetIndex, Download, GameArguments, LaunchEnvironment, Library, OsName, OsRule, Rule,
    RuleAction, Version, VersionDownloads, applicable_libraries, parse_version, rules_allow,
};

#[test]
fn an_absent_or_empty_rule_list_allows() {
    let env = environment(OsName::Linux, Arch::X86_64);

    assert!(rules_allow(None, &env));
    assert!(rules_allow(Some(&[]), &env));
}

#[test]
fn an_allow_rule_for_windows_does_not_match_linux() {
    let rules = [allow_os("windows")];

    assert!(rules_allow(
        Some(&rules),
        &environment(OsName::Windows, Arch::X86_64)
    ));
    assert!(!rules_allow(
        Some(&rules),
        &environment(OsName::Linux, Arch::X86_64)
    ));
}

#[test]
fn the_last_matching_rule_wins() {
    let windows = environment(OsName::Windows, Arch::X86_64);
    let allow_then_block = [allow_os("windows"), disallow_os("windows")];
    let block_then_allow = [disallow_os("windows"), allow_os("windows")];

    assert!(!rules_allow(Some(&allow_then_block), &windows));
    assert!(rules_allow(Some(&block_then_allow), &windows));
}

#[test]
fn architecture_selects_different_libraries() {
    let version = version_with(vec![
        library("x86-only", Some(vec![allow_arch("x86")])),
        library("x86-64-only", Some(vec![allow_arch("x86_64")])),
    ]);

    let x86 = names(&version, OsName::Linux, Arch::X86);
    let x86_64 = names(&version, OsName::Linux, Arch::X86_64);

    assert_eq!(x86, vec!["x86-only"]);
    assert_eq!(x86_64, vec!["x86-64-only"]);
}

#[test]
fn a_feature_rule_matches_only_when_that_feature_is_enabled() {
    let rules = [Rule {
        action: RuleAction::Allow,
        os: None,
        features: Some(BTreeMap::from([("is_demo_user".to_string(), true)])),
    }];
    let mut enabled = environment(OsName::Linux, Arch::X86_64);
    enabled.features.insert("is_demo_user".to_string(), true);
    let disabled = environment(OsName::Linux, Arch::X86_64);

    assert!(rules_allow(Some(&rules), &enabled));
    assert!(!rules_allow(Some(&rules), &disabled));
}

#[test]
fn an_os_version_pattern_must_match_and_an_invalid_pattern_does_not() {
    let mut current = environment(OsName::Osx, Arch::X86_64);
    current.os_version = "10.15.7".to_string();
    let matching = [allow_os_version(r"^10\.15")];
    let invalid = [allow_os_version("(")];

    assert!(rules_allow(Some(&matching), &current));
    assert!(!rules_allow(Some(&invalid), &current));
}

#[test]
fn saved_1_21_11_keeps_natives_for_the_requested_os() {
    let version = parse_version(include_str!("../fixtures/version-1.21.11.json"))
        .expect("the saved version should parse");

    let linux = names(&version, OsName::Linux, Arch::X86_64);
    let windows = names(&version, OsName::Windows, Arch::X86_64);

    assert!(linux.iter().any(|name| name.contains("natives-linux")));
    assert!(linux.iter().all(|name| !name.contains("natives-windows")));
    assert!(windows.iter().any(|name| name.contains("natives-windows")));
    assert!(windows.iter().all(|name| !name.contains("natives-linux")));
}

fn names(version: &Version, os: OsName, arch: Arch) -> Vec<String> {
    applicable_libraries(version, &environment(os, arch))
        .into_iter()
        .map(|library| library.name.clone())
        .collect()
}

fn environment(os: OsName, arch: Arch) -> LaunchEnvironment {
    LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    }
}

fn allow_os(name: &str) -> Rule {
    Rule {
        action: RuleAction::Allow,
        os: Some(os_name(name)),
        features: None,
    }
}

fn disallow_os(name: &str) -> Rule {
    Rule {
        action: RuleAction::Disallow,
        os: Some(os_name(name)),
        features: None,
    }
}

fn allow_arch(arch: &str) -> Rule {
    Rule {
        action: RuleAction::Allow,
        os: Some(OsRule {
            name: None,
            arch: Some(arch.to_string()),
            version: None,
        }),
        features: None,
    }
}

fn allow_os_version(version: &str) -> Rule {
    Rule {
        action: RuleAction::Allow,
        os: Some(OsRule {
            name: None,
            arch: None,
            version: Some(version.to_string()),
        }),
        features: None,
    }
}

fn os_name(name: &str) -> OsRule {
    OsRule {
        name: Some(name.to_string()),
        arch: None,
        version: None,
    }
}

fn library(name: &str, rules: Option<Vec<Rule>>) -> Library {
    Library {
        name: name.to_string(),
        downloads: None,
        natives: None,
        rules,
        extract: None,
    }
}

fn version_with(libraries: Vec<Library>) -> Version {
    Version {
        arguments: GameArguments::Legacy(String::new()),
        libraries,
        asset_index: AssetIndex {
            id: "1".to_string(),
            sha1: "a".repeat(40),
            size: 0,
            total_size: 0,
            url: "https://example.invalid/index.json".to_string(),
        },
        main_class: "a.B".to_string(),
        downloads: VersionDownloads {
            client: Download {
                sha1: "b".repeat(40),
                size: 0,
                url: "https://example.invalid/client.jar".to_string(),
            },
            server: None,
        },
        java_version: None,
    }
}
