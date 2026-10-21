use std::path::{Path, PathBuf};

use crate::{CoreError, Result};

pub(crate) const INSTALL_DIRECTORY: &str = ".install";

pub(crate) fn archive_path(instance: &Path) -> Result<PathBuf> {
    let directory = instance.join(INSTALL_DIRECTORY);
    match std::fs::symlink_metadata(&directory) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(CoreError::CachePath);
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(directory.join("pack.zip"))
}

pub(crate) fn cleanup(instance: &Path) -> Result<()> {
    let archive = archive_path(instance)?;
    let directory = archive.parent().ok_or(CoreError::CachePath)?;
    for name in ["pack.zip", "pack.zip.part"] {
        match std::fs::remove_file(directory.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    match std::fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {
            tracing::warn!("Preserving non-temporary files in an instance installation workspace");
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> PathBuf {
        std::env::temp_dir().join(format!(
            "licht-workspace-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[tokio::test]
    async fn cleanup_removes_only_the_archive_and_partial_and_is_repeatable() {
        let instance = instance();
        let archive = archive_path(&instance).expect("archive");
        std::fs::create_dir_all(archive.parent().expect("directory")).expect("workspace");
        std::fs::write(&archive, b"archive").expect("archive");
        std::fs::write(archive.with_file_name("pack.zip.part"), b"partial").expect("partial");
        std::fs::write(instance.join("save.dat"), b"world").expect("save");
        cleanup(&instance).expect("cleanup");
        cleanup(&instance).expect("repeat cleanup");
        assert!(!instance.join(INSTALL_DIRECTORY).exists());
        assert_eq!(
            std::fs::read(instance.join("save.dat")).expect("save"),
            b"world"
        );
    }

    #[tokio::test]
    async fn cleanup_preserves_unknown_files_inside_the_reserved_directory() {
        let instance = instance();
        let archive = archive_path(&instance).expect("archive");
        std::fs::create_dir_all(archive.parent().expect("directory")).expect("workspace");
        let other = archive.with_file_name("unknown.txt");
        std::fs::write(&other, b"preserve").expect("unknown file");
        cleanup(&instance).expect("cleanup");
        assert_eq!(std::fs::read(other).expect("preserved file"), b"preserve");
    }
}
