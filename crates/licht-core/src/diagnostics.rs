use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BuildDiagnostics {
    pub host_version: String,
    pub host_revision: Option<String>,
    pub debug: bool,
    pub tauri_version: String,
    pub webview_version: Option<String>,
    pub os_name: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub system_architecture: String,
    pub process_architecture: String,
}

pub fn build_diagnostics(
    host_version: &str,
    host_revision: Option<&str>,
    tauri_version: &str,
    webview_version: Option<String>,
    debug: bool,
) -> BuildDiagnostics {
    BuildDiagnostics {
        host_version: host_version.to_string(),
        host_revision: host_revision.map(str::to_string),
        debug,
        tauri_version: tauri_version.to_string(),
        webview_version,
        os_name: sysinfo::System::long_os_version()
            .filter(|name| !name.is_empty())
            .or_else(sysinfo::System::name)
            .unwrap_or_else(|| std::env::consts::OS.to_string()),
        os_version: operating_system_version(),
        kernel_version: sysinfo::System::kernel_version(),
        system_architecture: sysinfo::System::cpu_arch(),
        process_architecture: std::env::consts::ARCH.to_string(),
    }
}

#[cfg(windows)]
fn operating_system_version() -> Option<String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    // sysinfo exposes the marketing version, e.g. "11 (26300)", rather than NT's version.
    match std::process::Command::new("cmd")
        .args(["/d", "/c", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) if output.status.success() => {
            if let Some(version) = parse_windows_version(&String::from_utf8_lossy(&output.stdout)) {
                return Some(version);
            }
            tracing::warn!("Windows version output was not recognized; using the build number");
        }
        Ok(output) => {
            tracing::warn!(status = %output.status, "Windows version query failed; using the build number")
        }
        Err(error) => {
            tracing::warn!(%error, "Could not query Windows version; using the build number")
        }
    }
    sysinfo::System::kernel_version().map(|build| format!("build {build}"))
}

#[cfg(not(windows))]
fn operating_system_version() -> Option<String> {
    sysinfo::System::os_version()
}

#[cfg(any(windows, test))]
fn parse_windows_version(output: &str) -> Option<String> {
    output
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .find_map(|token| {
            let parts = token.split('.').collect::<Vec<_>>();
            if (3..=4).contains(&parts.len()) && parts.iter().all(|part| !part.is_empty()) {
                Some(parts[..3].join("."))
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_technical_version_comes_from_ver_without_the_revision() {
        assert_eq!(
            parse_windows_version("\r\nMicrosoft Windows [versão 10.0.26300.9711]\r\n").as_deref(),
            Some("10.0.26300")
        );
        assert_eq!(
            parse_windows_version("Microsoft Windows [Version 10.0.26100.0]").as_deref(),
            Some("10.0.26100")
        );
        assert_eq!(
            parse_windows_version("Microsoft Windows [Version 6.1.7601]").as_deref(),
            Some("6.1.7601")
        );
    }

    #[test]
    fn incomplete_or_unexpected_windows_versions_are_not_guessed() {
        for output in [
            "",
            "11 (26300)",
            "Windows 11 Pro",
            "Version 10.0",
            "Version 10..26300",
            "Version 10.0.26300.0.1",
        ] {
            assert!(parse_windows_version(output).is_none(), "{output}");
        }
    }

    #[test]
    fn collects_runtime_information_and_keeps_unavailable_values_explicit() {
        let diagnostics = build_diagnostics("0.3.0", Some("abc1234"), "2.12.1", None, true);
        assert_eq!(diagnostics.host_version, "0.3.0");
        assert_eq!(diagnostics.host_revision.as_deref(), Some("abc1234"));
        assert_eq!(diagnostics.tauri_version, "2.12.1");
        assert!(diagnostics.webview_version.is_none());
        assert!(diagnostics.debug);
        assert!(!diagnostics.os_name.is_empty());
        assert!(!diagnostics.system_architecture.is_empty());
        assert_eq!(diagnostics.process_architecture, std::env::consts::ARCH);
        #[cfg(windows)]
        assert!(
            diagnostics
                .os_version
                .as_deref()
                .is_some_and(|version| version.split('.').count() == 3)
        );
    }
}
