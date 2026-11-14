use licht_cli::commands::instances::{
    InstanceRef, InstanceResolveError, InstancesListOutput, format_instances_list,
    format_instances_table, resolve_instance,
};
use licht_core::InstanceEntry;

fn auto(interactive: bool) -> InstancesListOutput {
    InstancesListOutput {
        json: false,
        interactive,
    }
}

fn entry(folder: &str, name: &str, version: Option<&str>) -> InstanceEntry {
    InstanceEntry {
        folder: folder.into(),
        name: name.into(),
        version_id: version.map(str::to_string),
        installation: None,
        min_memory_mb: 512,
        max_memory_mb: 2048,
        jvm_arguments: Vec::new(),
        fullscreen: false,
        width: None,
        height: None,
        override_window: false,
        override_memory: false,
        override_java: false,
        override_jvm_arguments: false,
        java_path: None,
        last_opened_at: None,
        icon_path: None,
    }
}

#[test]
fn resolve_prefers_an_exact_folder() {
    let entries = [
        InstanceRef {
            folder: "Survival".into(),
            name: "Old Name".into(),
        },
        InstanceRef {
            folder: "other".into(),
            name: "Survival".into(),
        },
    ];

    let resolved = resolve_instance("Survival", &entries).expect("folder match");
    assert_eq!(resolved.folder, "Survival");
}

#[test]
fn resolve_accepts_a_unique_display_name() {
    let entries = [InstanceRef {
        folder: "survival-1".into(),
        name: "Survival".into(),
    }];

    let resolved = resolve_instance("Survival", &entries).expect("name match");
    assert_eq!(resolved.folder, "survival-1");
}

#[test]
fn resolve_rejects_an_ambiguous_display_name() {
    let entries = [
        InstanceRef {
            folder: "survival-1".into(),
            name: "Survival".into(),
        },
        InstanceRef {
            folder: "survival-2".into(),
            name: "Survival".into(),
        },
    ];

    assert_eq!(
        resolve_instance("Survival", &entries),
        Err(InstanceResolveError::Ambiguous {
            folders: vec!["survival-1".into(), "survival-2".into()],
        })
    );
}

#[test]
fn resolve_rejects_a_missing_instance() {
    assert_eq!(
        resolve_instance("Missing", &[]),
        Err(InstanceResolveError::Missing)
    );
}

#[test]
fn non_interactive_list_uses_tabs_and_a_dash_for_missing_versions() {
    let entries = [entry("survival", "Survival", None)];

    assert_eq!(
        format_instances_list(&entries, auto(false)),
        vec!["survival\tSurvival\t-".to_string()]
    );
}

#[test]
fn interactive_list_prints_an_aligned_table_with_headers() {
    let entries = [
        entry(
            "Cobblemon-Official-Modpack-Fabric",
            "Cobblemon Official Modpack [Fabric]",
            Some("fabric-loader-0.20.1-1.21.1"),
        ),
        entry(
            "Fabulously-Optimized",
            "Fabulously Optimized",
            Some("fabric-loader-0.20.1-26.3"),
        ),
    ];

    assert_eq!(
        format_instances_table(&entries, false),
        vec![
            "┌───────────────────────────────────┬─────────────────────────────────────┬─────────────────────────────┐".to_string(),
            "│ FOLDER                            │ NAME                                │ VERSION                     │".to_string(),
            "├───────────────────────────────────┼─────────────────────────────────────┼─────────────────────────────┤".to_string(),
            "│ Cobblemon-Official-Modpack-Fabric │ Cobblemon Official Modpack [Fabric] │ fabric-loader-0.20.1-1.21.1 │".to_string(),
            "│ Fabulously-Optimized              │ Fabulously Optimized                │ fabric-loader-0.20.1-26.3   │".to_string(),
            "└───────────────────────────────────┴─────────────────────────────────────┴─────────────────────────────┘".to_string(),
        ]
    );

    let colored = format_instances_list(&entries, auto(true));
    assert_eq!(colored.len(), 6);
    assert!(colored[0].starts_with("\u{1b}[90m┌"));
    assert!(colored[1].starts_with("\u{1b}[90m│\u{1b}[0m \u{1b}[1;33mFOLDER"));
    assert!(colored[1].contains("\u{1b}[90m│\u{1b}[0m"));
    assert!(colored[2].contains('┼'));
    assert!(colored[3].contains("\u{1b}[90mfabric-loader-0.20.1-1.21.1\u{1b}[0m"));
    assert!(colored[3].starts_with("\u{1b}[90m│\u{1b}[0m "));
    assert!(colored[5].contains('┘'));
}

#[test]
fn json_list_prints_stable_objects() {
    let entries = [
        entry("survival", "Survival", Some("1.20.1")),
        entry("empty", "Empty", None),
    ];

    assert_eq!(
        format_instances_list(
            &entries,
            InstancesListOutput {
                json: true,
                interactive: true,
            }
        ),
        vec![
            "[\n  {\n    \"folder\": \"survival\",\n    \"name\": \"Survival\",\n    \"versionId\": \"1.20.1\"\n  },\n  {\n    \"folder\": \"empty\",\n    \"name\": \"Empty\",\n    \"versionId\": null\n  }\n]"
                .to_string()
        ]
    );
}

#[test]
fn json_list_escapes_quotes_in_names() {
    let entries = [entry("pack", "Name \"quoted\"", Some("1.0"))];

    assert_eq!(
        format_instances_list(
            &entries,
            InstancesListOutput {
                json: true,
                interactive: false,
            }
        ),
        vec![
            "[\n  {\n    \"folder\": \"pack\",\n    \"name\": \"Name \\\"quoted\\\"\",\n    \"versionId\": \"1.0\"\n  }\n]"
                .to_string()
        ]
    );
}

#[test]
fn interactive_list_is_empty_when_there_are_no_instances() {
    assert!(format_instances_list(&[], auto(true)).is_empty());
}
