use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::{CoreError, OsName, Result};

pub(crate) fn extract_java_archive(archive: &Path, destination: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.ends_with(".tar.gz") {
        extract_tar_gz(archive, destination)
    } else if name.ends_with(".zip") {
        extract_zip(archive, destination)
    } else {
        Err(CoreError::JavaRuntimeMissing)
    }
}

fn extract_zip(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(std::io::Error::other)?;
        let Some(enclosed) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            return Err(CoreError::CachePath);
        };
        let relative = archive_relative(&enclosed)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let relative = archive_relative(&path)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if kind.is_symlink() {
            let Some(link) = entry.link_name()? else {
                return Err(CoreError::CachePath);
            };
            let Some(link) = link.to_str() else {
                return Err(CoreError::CachePath);
            };
            place_link(&target, link)?;
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

/// Drops the archive's top directory, so `jdk-21/bin/java` lands at `bin/java`.
pub(crate) fn archive_relative(path: &Path) -> Result<PathBuf> {
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Normal(_)) => {}
        _ => return Err(CoreError::CachePath),
    }
    let mut relative = PathBuf::new();
    for component in parts {
        match component {
            Component::Normal(name) => relative.push(name),
            _ => return Err(CoreError::CachePath),
        }
    }
    Ok(relative)
}

pub(crate) fn java_executable(root: &Path, os: OsName) -> PathBuf {
    match os {
        OsName::Windows => root.join("bin").join("java.exe"),
        OsName::Linux | OsName::Osx => root.join("bin").join("java"),
    }
}

pub(crate) fn under(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty() {
        return Err(CoreError::CachePath);
    }
    let mut full = root.to_path_buf();
    for component in relative.split(['/', '\\']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(CoreError::CachePath);
        }
        full.push(component);
    }
    Ok(full)
}

pub(crate) fn file_matches(path: &Path, expected_sha1: &str) -> Result<bool> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(hasher.finalize().as_slice()).eq_ignore_ascii_case(expected_sha1))
}

pub(crate) fn mark_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        std::fs::set_permissions(path, permissions)?;
    }
    let _ = path;
    Ok(())
}

pub(crate) fn place_link(link: &Path, target: &str) -> Result<()> {
    if link.exists() {
        std::fs::remove_file(link)?;
    }
    let relative = Path::new(target);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(relative, link)?;
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_file(relative, link).is_err() {
            tracing::warn!("Java runtime symlink was copied because the link could not be created");
            let Some(parent) = link.parent() else {
                return Err(CoreError::CachePath);
            };
            std::fs::copy(parent.join(relative), link)?;
        }
    }
    Ok(())
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{archive_relative, extract_java_archive};

    #[test]
    fn an_archive_path_outside_the_runtime_is_rejected() {
        assert!(archive_relative(std::path::Path::new("jdk-21/../outside")).is_err());
    }

    #[test]
    fn a_gzip_archive_drops_its_top_directory() {
        let directory =
            std::env::temp_dir().join(format!("licht-temurin-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        let archive = directory.join("temurin.tar.gz");
        std::fs::write(&archive, sample_tar_gz()).expect("archive");
        extract_java_archive(&archive, &directory).expect("extract");
        assert_eq!(
            std::fs::read(directory.join("bin").join("java")).expect("java"),
            b"temurin"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    fn sample_tar_gz() -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_path("jdk-21/bin/java").expect("path");
        header.set_size(7);
        header.set_mode(0o755);
        header.set_cksum();
        let mut builder = tar::Builder::new(Vec::new());
        builder.append(&header, &b"temurin"[..]).expect("tar entry");
        let tar_bytes = builder.into_inner().expect("tar");
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar_bytes).expect("gzip");
        encoder.finish().expect("gzip finish")
    }
}
