use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use sha1::{Digest, Sha1};

use super::progress::StartupReporter;
use crate::{CoreError, Result};

pub(super) fn digest(path: &Path, reporter: &StartupReporter) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        reporter.checkpoint()?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_vec())
}

pub(super) fn copy_tree_with_progress(
    source: &Path,
    destination: &Path,
    reporter: &StartupReporter,
) -> Result<()> {
    let mut progress = CopyProgress {
        reporter: reporter.clone(),
        files: 0,
        bytes: 0,
        last: std::time::Instant::now(),
    };
    copy_tree_inner(source, destination, &mut progress)?;
    if progress.files > 0 {
        tracing::info!(
            verified_files = progress.files,
            verified_bytes = progress.bytes,
            "Storage migration copy batch verified"
        );
    }
    Ok(())
}

struct CopyProgress {
    reporter: StartupReporter,
    files: u64,
    bytes: u64,
    last: std::time::Instant,
}

fn copy_tree_inner(source: &Path, destination: &Path, progress: &mut CopyProgress) -> Result<()> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => return Err(CoreError::CachePath),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            copy_tree_inner(&item.path(), &destination.join(item.file_name()), progress)?;
        }
    } else if metadata.is_file() {
        copy_file_with_progress(source, destination, &progress.reporter)?;
        progress.files += 1;
        progress.bytes += metadata.len();
        if progress.last.elapsed() >= std::time::Duration::from_secs(1) {
            tracing::info!(
                verified_files = progress.files,
                verified_bytes = progress.bytes,
                "Storage migration is copying and verifying files"
            );
            progress.last = std::time::Instant::now();
        }
    } else {
        return Err(CoreError::CachePath);
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    copy_file_with_progress(source, destination, &StartupReporter::default())
}

pub(super) fn copy_file_with_progress(
    source: &Path,
    destination: &Path,
    reporter: &StartupReporter,
) -> Result<()> {
    let before = fs::symlink_metadata(source)?;
    if !before.is_file() {
        return Err(CoreError::CachePath);
    }
    let destination_metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if let Some(metadata) = destination_metadata {
        let expected = digest(source, reporter)?;
        if !metadata.is_file() || digest(destination, reporter)? != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "launcher migration destination contains different data",
            )
            .into());
        }
        if !before.permissions().readonly()
            && fs::metadata(destination)?.modified()? != before.modified()?
        {
            fs::File::options()
                .write(true)
                .open(destination)?
                .set_times(fs::FileTimes::new().set_modified(before.modified()?))?;
        }
        fs::set_permissions(destination, before.permissions())?;
        return reporter.file(before.len());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|error| std::io::Error::other(error.to_string()))?;
    let temporary = destination
        .parent()
        .ok_or(CoreError::CachePath)?
        .join(format!(
            ".licht-layout-{}",
            crate::hashes::hex_encode(&nonce)
        ));
    let result = (|| -> Result<()> {
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let mut expected = Sha1::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            reporter.checkpoint()?;
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            expected.update(&buffer[..read]);
        }
        output.flush()?;
        output.set_times(fs::FileTimes::new().set_modified(before.modified()?))?;
        output.sync_all()?;
        drop(output);
        let after = fs::metadata(source)?;
        if before.len() != after.len()
            || before.modified()? != after.modified()?
            || digest(&temporary, reporter)? != expected.finalize().to_vec()
        {
            return Err(
                std::io::Error::other("launcher migration copy verification failed").into(),
            );
        }
        fs::rename(&temporary, destination)?;
        fs::set_permissions(destination, before.permissions())?;
        Ok(())
    })();
    if temporary.exists()
        && let Err(error) = fs::remove_file(&temporary)
    {
        if result.is_ok() {
            return Err(error.into());
        }
        tracing::warn!("Could not remove a failed migration temporary copy");
    }
    result?;
    reporter.file(before.len())
}
