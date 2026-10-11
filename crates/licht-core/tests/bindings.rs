use std::path::{Path, PathBuf};

use licht_core::{
    DownloadProgress, GameExit, GameLine, GameLogEvent, InstanceEntry, InstanceSettings,
    JavaDetections, JavaPaths, JavaStatus, Settings, SettingsSnapshot, VersionSummary,
};
use ts_rs::TS;

fn generated_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/lib/generated")
}

fn read_normalized(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n")
}

fn file_names(directory: &Path) -> Vec<String> {
    let mut names = std::fs::read_dir(directory)
        .expect("bindings directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn the_committed_bindings_match_ts_rs() {
    let actual = std::env::temp_dir().join(format!("licht-bindings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&actual);
    std::fs::create_dir_all(&actual).expect("temp bindings");
    let config = ts_rs::Config::new().with_out_dir(&actual);
    VersionSummary::export_all(&config).expect("version summary");
    InstanceEntry::export_all(&config).expect("instance entry");
    InstanceSettings::export_all(&config).expect("instance settings");
    DownloadProgress::export_all(&config).expect("progress");
    GameLine::export_all(&config).expect("game line");
    GameLogEvent::export_all(&config).expect("game log event");
    GameExit::export_all(&config).expect("game exit");
    Settings::export_all(&config).expect("settings");
    SettingsSnapshot::export_all(&config).expect("settings snapshot");
    JavaDetections::export_all(&config).expect("java detections");
    JavaPaths::export_all(&config).expect("java paths");
    JavaStatus::export_all(&config).expect("java status");

    let expected = generated_dir();
    let expected_names = file_names(&expected);
    let actual_names = file_names(&actual);
    assert_eq!(
        expected_names,
        actual_names,
        "run the export and commit src/lib/generated. exported to {}",
        actual.display()
    );
    for name in expected_names {
        assert_eq!(
            read_normalized(&expected.join(&name)),
            read_normalized(&actual.join(&name)),
            "{name} is stale. exported to {}",
            actual.display()
        );
    }
    let _ = std::fs::remove_dir_all(actual);
}
