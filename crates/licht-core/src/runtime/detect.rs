use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::archive::java_executable;
use super::temurin::find_temurin;
use crate::{CoreError, LaunchEnvironment, OsName, Result, SharedCache};

const JAVA_SEARCH_DEPTH: u32 = 4;

pub(crate) fn java_binary_name(os: OsName) -> &'static str {
    match os {
        OsName::Windows => "java.exe",
        OsName::Linux | OsName::Osx => "java",
    }
}

/// Usual install directories. A missing directory is not an error.
pub fn default_java_roots(os: OsName) -> &'static [&'static str] {
    match os {
        OsName::Windows => &[
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\BellSoft",
            r"C:\Program Files\Zulu",
        ],
        OsName::Linux => &["/usr/lib/jvm", "/usr/java"],
        OsName::Osx => &[
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ],
    }
}

/// Major version printed by `java -version`.
///
/// Java 8 reports `1.8.0_51`. Java 9 and later report `21.0.7`.
pub fn parse_java_version(output: &str) -> Result<u32> {
    for line in output.lines() {
        let Some(version) = quoted_version(line) else {
            continue;
        };
        return major_from_version(version);
    }
    Err(CoreError::JavaProbe)
}

pub fn probe_java_major(executable: &Path) -> Result<u32> {
    if let Some(major) = major_from_release_file(executable) {
        return Ok(major);
    }
    let output = std::process::Command::new(executable)
        .arg("-version")
        .output()?;
    if !output.status.success() {
        return Err(CoreError::JavaProbe);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    parse_java_version(&text)
}

pub fn validate_java(executable: &Path, required: &crate::JavaVersion) -> Result<u32> {
    let found = probe_java_major(executable)?;
    if found != required.major_version {
        return Err(CoreError::JavaMajor {
            found,
            required: required.major_version,
        });
    }
    Ok(found)
}

/// Accepts an existing JVM for `--java` / custom install paths (no download).
pub fn use_custom_java(executable: &Path) -> Result<PathBuf> {
    if !executable.is_file() {
        return Err(CoreError::JavaPath);
    }
    Ok(executable.to_path_buf())
}

/// `java` / `java.exe` under each root, including the usual `bin` and macOS `.jdk` layouts.
pub fn discover_javas(roots: &[PathBuf], os: OsName) -> Vec<PathBuf> {
    let name = java_binary_name(os);
    let mut found = Vec::new();
    for root in roots {
        collect_javas(root, name, 0, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaDetections {
    pub java25: Vec<String>,
    pub java21: Vec<String>,
    pub java17: Vec<String>,
    pub java8: Vec<String>,
}

/// Executable already stored under `runtime/` for each major the settings screen offers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaPaths {
    pub java25: Option<String>,
    pub java21: Option<String>,
    pub java17: Option<String>,
    pub java8: Option<String>,
}

/// Whether each path is a file of that major.
///
/// The JDK `release` file answers this without starting a JVM. `java -version`
/// runs only when that file is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JavaStatus {
    pub java25: bool,
    pub java21: bool,
    pub java17: bool,
    pub java8: bool,
}

/// System install folders plus this launcher's `runtime` directory.
pub fn java_detect_roots(cache: &SharedCache, os: OsName) -> Vec<PathBuf> {
    let mut roots = default_java_roots(os)
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    roots.push(cache.root().join("runtime"));
    roots
}

/// Executables `discover_javas` finds, grouped by the majors the settings screen offers.
pub fn detect_javas(os: OsName) -> JavaDetections {
    let roots = default_java_roots(os)
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    group_detected(&roots, os)
}

/// Same search as [`detect_javas`], and also the Javas this launcher installed.
pub fn detect_known_javas(cache: &SharedCache, os: OsName) -> JavaDetections {
    group_detected(&java_detect_roots(cache, os), os)
}

fn group_detected(roots: &[PathBuf], os: OsName) -> JavaDetections {
    let mut found = JavaDetections::default();
    for path in discover_javas(roots, os) {
        let Ok(major) = probe_java_major(&path) else {
            continue;
        };
        let text = path.display().to_string();
        match major {
            25 => found.java25.push(text),
            21 => found.java21.push(text),
            17 => found.java17.push(text),
            8 => found.java8.push(text),
            _ => {}
        }
    }
    found
}

/// Java already installed in this cache's `runtime` folder. A missing major stays empty.
pub fn runtime_java_paths(
    cache: &SharedCache,
    environment: &LaunchEnvironment,
) -> Result<JavaPaths> {
    Ok(JavaPaths {
        java25: runtime_executable(cache, 25, environment)?,
        java21: runtime_executable(cache, 21, environment)?,
        java17: runtime_executable(cache, 17, environment)?,
        java8: runtime_executable(cache, 8, environment)?,
    })
}

fn runtime_executable(
    cache: &SharedCache,
    major: u32,
    environment: &LaunchEnvironment,
) -> Result<Option<String>> {
    let Some(root) = find_temurin(cache, major, environment)? else {
        return Ok(None);
    };
    let executable = java_executable(&root, environment.os);
    if executable.is_file() {
        Ok(Some(executable.display().to_string()))
    } else {
        Ok(None)
    }
}

/// Folder a file picker should open.
///
/// Uses the directory of `current` when that directory exists. An empty path,
/// or one whose directory is missing, opens `fallback`.
pub fn browse_start_directory(current: &str, fallback: &Path) -> PathBuf {
    let current = current.trim();
    if !current.is_empty() {
        let path = Path::new(current);
        if path.is_dir() {
            return path.to_path_buf();
        }
        if let Some(parent) = path.parent()
            && parent.is_dir()
        {
            return parent.to_path_buf();
        }
    }
    fallback.to_path_buf()
}

pub fn java_installation_status(paths: &JavaPaths) -> JavaStatus {
    JavaStatus {
        java25: java_matches(paths.java25.as_deref(), 25),
        java21: java_matches(paths.java21.as_deref(), 21),
        java17: java_matches(paths.java17.as_deref(), 17),
        java8: java_matches(paths.java8.as_deref(), 8),
    }
}

fn java_matches(path: Option<&str>, major: u32) -> bool {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let path = Path::new(path);
    if !path.is_file() {
        return false;
    }
    probe_java_major(path).ok() == Some(major)
}

/// First executable whose major version equals `required`. Others are skipped.
pub fn matching_java(executables: &[PathBuf], required: &crate::JavaVersion) -> Option<PathBuf> {
    for executable in executables {
        match probe_java_major(executable) {
            Ok(major) if major == required.major_version => return Some(executable.clone()),
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    path = %executable.display(),
                    "installed Java could not be read: {error}"
                );
            }
        }
    }
    None
}

fn quoted_version(line: &str) -> Option<&str> {
    let marker = "version \"";
    let start = line.find(marker)? + marker.len();
    let rest = line.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

fn major_from_release_file(executable: &Path) -> Option<u32> {
    let parent = executable.parent()?;
    let home = if parent.file_name().is_some_and(|name| name == "bin") {
        parent.parent()?
    } else {
        parent
    };
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    for line in text.lines() {
        let Some(version) = line.trim().strip_prefix("JAVA_VERSION=") else {
            continue;
        };
        let version = version.trim().trim_matches('"');
        if let Ok(major) = major_from_version(version) {
            return Some(major);
        }
    }
    None
}

fn major_from_version(version: &str) -> Result<u32> {
    let mut parts = version.split('.');
    let Some(first) = parts.next().filter(|part| !part.is_empty()) else {
        return Err(CoreError::JavaProbe);
    };
    let component = if first == "1" {
        parts.next().ok_or(CoreError::JavaProbe)?
    } else {
        first
    };
    let digits: String = component
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return Err(CoreError::JavaProbe);
    }
    digits.parse().map_err(|_| CoreError::JavaProbe)
}

fn collect_javas(dir: &Path, name: &str, depth: u32, found: &mut Vec<PathBuf>) {
    let candidate = dir.join(name);
    if candidate.is_file() {
        found.push(candidate);
    }
    if depth == JAVA_SEARCH_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            tracing::warn!(path = %dir.display(), "Java directory could not be read: {error}");
            return;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            collect_javas(&path, name, depth + 1, found);
        }
    }
}
