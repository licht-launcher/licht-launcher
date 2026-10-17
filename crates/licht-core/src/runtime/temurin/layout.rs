use std::path::PathBuf;

use super::super::archive::java_executable;
use crate::{Arch, CoreError, LaunchEnvironment, OsName, Result, SharedCache};

pub(super) fn runtime_folder(cache: &SharedCache, name: &str) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(CoreError::CachePath);
    }
    Ok(cache.root().join("runtime").join(name))
}

/// `temurin25-jre25.0.2-win_x64`: one folder, with the package and the machine in the name.
pub(crate) fn temurin_folder_name(
    major: u32,
    image: &str,
    version: &str,
    os: OsName,
    arch: Arch,
) -> Result<String> {
    if image != "jre" && image != "jdk" {
        return Err(CoreError::JavaRuntimeMissing);
    }
    if version.is_empty()
        || version.contains(['/', '\\', '-'])
        || version
            .split('.')
            .any(|part| part.is_empty() || part.parse::<u32>().is_err())
    {
        return Err(CoreError::JavaRuntimeMissing);
    }
    Ok(format!(
        "temurin{major}-{image}{version}-{}",
        platform_label(os, arch)
    ))
}

pub(crate) fn platform_label(os: OsName, arch: Arch) -> String {
    let os = match os {
        OsName::Windows => "win",
        OsName::Linux => "linux",
        OsName::Osx => "mac",
    };
    let arch = match arch {
        Arch::X86_64 => "x64",
        Arch::X86 => "x86",
    };
    format!("{os}_{arch}")
}

pub(crate) fn find_temurin(
    cache: &SharedCache,
    major: u32,
    environment: &LaunchEnvironment,
) -> Result<Option<PathBuf>> {
    let runtime = cache.root().join("runtime");
    let platform = platform_label(environment.os, environment.arch);
    let entries = match std::fs::read_dir(&runtime) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if folder_middle(&name, major, &platform).is_none() {
            continue;
        }
        if java_executable(&path, environment.os).is_file() && !path.join(".installing").exists() {
            found.push((name, path));
        }
    }
    found.sort_by_key(|left| std::cmp::Reverse(temurin_rank(&left.0)));
    Ok(found.into_iter().next().map(|(_, path)| path))
}

fn folder_middle<'a>(name: &'a str, major: u32, platform: &str) -> Option<&'a str> {
    let rest = name.strip_prefix(&format!("temurin{major}-"))?;
    let middle = rest.strip_suffix(&format!("-{platform}"))?;
    if middle.starts_with("jre") || middle.starts_with("jdk") {
        Some(middle)
    } else {
        None
    }
}

fn temurin_rank(name: &str) -> (bool, Vec<u32>) {
    let middle = name.split('-').nth(1).unwrap_or("");
    let (jre, version) = if let Some(version) = middle.strip_prefix("jre") {
        (true, version)
    } else if let Some(version) = middle.strip_prefix("jdk") {
        (false, version)
    } else {
        (false, "")
    };
    let parts = version
        .split('.')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    (jre, parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_temurin_folder_names_the_package_and_the_machine() {
        assert_eq!(
            temurin_folder_name(25, "jre", "25.0.2", OsName::Windows, Arch::X86_64).expect("name"),
            "temurin25-jre25.0.2-win_x64"
        );
        assert_eq!(
            temurin_folder_name(8, "jdk", "8.0.422", OsName::Linux, Arch::X86).expect("name"),
            "temurin8-jdk8.0.422-linux_x86"
        );
    }

    #[test]
    fn an_executable_from_an_interrupted_extraction_is_not_an_installed_runtime() {
        let root =
            std::env::temp_dir().join(format!("licht-runtime-incomplete-{}", std::process::id()));
        let cache = SharedCache::at(&root);
        let runtime = root.join("runtime").join("temurin21-jre21.0.1-linux_x64");
        std::fs::create_dir_all(runtime.join("bin")).expect("runtime folder");
        std::fs::write(runtime.join("bin").join("java"), b"partial extraction")
            .expect("executable");
        std::fs::write(runtime.join(".installing"), b"").expect("marker");
        let environment = LaunchEnvironment {
            os: OsName::Linux,
            arch: Arch::X86_64,
            os_version: String::new(),
            features: Default::default(),
        };
        assert_eq!(
            find_temurin(&cache, 21, &environment).expect("pending"),
            None
        );
        std::fs::remove_file(runtime.join(".installing")).expect("finish extraction");
        assert_eq!(
            find_temurin(&cache, 21, &environment).expect("ready"),
            Some(runtime)
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
