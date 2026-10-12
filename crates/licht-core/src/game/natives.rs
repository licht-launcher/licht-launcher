use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::{
    Arch, Artifact, CoreError, LaunchEnvironment, Library, OsName, Result, SharedCache, Version,
    applicable_libraries,
};

pub struct NativeLibrary<'a> {
    pub artifact: &'a Artifact,
    pub exclude: &'a [String],
}

/// Native jars for this OS and architecture.
///
/// Old versions name the jar in `natives`, and `${arch}` becomes `32` or `64`.
/// Current versions put `natives-windows` in the library name. The plain name is
/// 64-bit; `natives-windows-x86` and `natives-windows-arm64` are other machines.
pub fn native_libraries<'a>(
    version: &'a Version,
    env: &LaunchEnvironment,
) -> Vec<NativeLibrary<'a>> {
    let mut libraries = Vec::new();
    for library in applicable_libraries(version, env) {
        if let Some(native) = legacy_native(library, env) {
            libraries.push(native);
            continue;
        }
        if let Some(native) = modern_native(library, env) {
            libraries.push(native);
        }
    }
    libraries
}

pub fn extract_natives(
    cache: &SharedCache,
    version: &Version,
    env: &LaunchEnvironment,
    destination: &Path,
) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for library in native_libraries(version, env) {
        let path = cache.library(&library.artifact.path)?;
        extract_archive(&path, destination, library.exclude)?;
    }
    Ok(())
}

/// `natives/<version>` inside the cache. Binaries for this machine are extracted here.
pub fn create_natives_directory(cache: &SharedCache, version_id: &str) -> Result<PathBuf> {
    let path = cache.natives_dir(version_id)?;
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Old native maps store `natives-windows-${arch}`. `${arch}` is `64` or `32`.
pub(crate) fn native_classifier(template: &str, arch: Arch) -> String {
    template.replace("${arch}", arch_bits(arch))
}

fn legacy_native<'a>(library: &'a Library, env: &LaunchEnvironment) -> Option<NativeLibrary<'a>> {
    let template = library.natives.as_ref()?.get(env.os.mojang_name())?;
    let classifier = native_classifier(template, env.arch);
    let artifact = library
        .downloads
        .as_ref()?
        .classifiers
        .as_ref()?
        .get(&classifier)?;
    Some(NativeLibrary {
        artifact,
        exclude: exclude_list(library),
    })
}

fn modern_native<'a>(library: &'a Library, env: &LaunchEnvironment) -> Option<NativeLibrary<'a>> {
    let classifier = library.name.split(':').nth(3)?;
    if !classifier_matches(classifier, env.os, env.arch) {
        return None;
    }
    let artifact = library.downloads.as_ref()?.artifact.as_ref()?;
    Some(NativeLibrary {
        artifact,
        exclude: exclude_list(library),
    })
}

pub(crate) fn classifier_matches(classifier: &str, os: OsName, arch: Arch) -> bool {
    let Some(rest) = classifier.strip_prefix("natives-") else {
        return false;
    };
    let (platform, suffix) = match rest.split_once('-') {
        Some((platform, suffix)) => (platform, Some(suffix)),
        None => (rest, None),
    };
    let platform_matches = match os {
        OsName::Windows => platform == "windows",
        OsName::Linux => platform == "linux",
        OsName::Osx => platform == "osx" || platform == "macos",
    };
    if !platform_matches {
        return false;
    }
    match arch {
        Arch::X86_64 => suffix.is_none(),
        Arch::X86 => suffix == Some("x86"),
    }
}

fn arch_bits(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "64",
        Arch::X86 => "32",
    }
}

fn exclude_list(library: &Library) -> &[String] {
    library
        .extract
        .as_ref()
        .map(|extract| extract.exclude.as_slice())
        .unwrap_or(&[])
}

fn extract_archive(jar: &Path, destination: &Path, exclude: &[String]) -> Result<()> {
    let file = File::open(jar)?;
    let mut archive = zip::ZipArchive::new(file).map_err(CoreError::NativeArchive)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(CoreError::NativeArchive)?;
        let name = entry.name().replace('\\', "/");
        if entry.is_dir() || is_excluded(&name, exclude) {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            return Err(CoreError::NativePath);
        };
        if !is_shared_library(&relative) {
            continue;
        }
        let Some(file_name) = relative.file_name() else {
            return Err(CoreError::NativePath);
        };
        let mut output = File::create(destination.join(file_name))?;
        io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn is_excluded(name: &str, exclude: &[String]) -> bool {
    exclude.iter().any(|prefix| name.starts_with(prefix))
}

fn is_shared_library(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("dll")
            || extension.eq_ignore_ascii_case("so")
            || extension.eq_ignore_ascii_case("dylib")
    })
}
