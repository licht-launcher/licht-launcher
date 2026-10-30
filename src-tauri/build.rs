fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    for path in ["../.git/HEAD", "../.git/refs/heads", "../.git/packed-refs"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let revision = std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| {
            value.len() >= 7
                && value.len() <= 64
                && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .or_else(|| {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
            (value.len() >= 7
                && value.len() <= 64
                && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then_some(value)
        });
    if let Some(revision) = revision {
        println!("cargo:rustc-env=LICHT_BUILD_REVISION={}", &revision[..7]);
    } else {
        println!("cargo:warning=Native build revision is unavailable; diagnostics will show N/A");
    }
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
