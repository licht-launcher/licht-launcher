use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::{CoreError, Result};

fn digest(path: &Path) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_vec())
}

pub(super) fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let mut progress = CopyProgress {
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
    files: u64,
    bytes: u64,
    last: std::time::Instant,
}

fn copy_tree_inner(source: &Path, destination: &Path, progress: &mut CopyProgress) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(source)?;
    if destination.exists() && fs::symlink_metadata(destination)?.file_type().is_symlink() {
        return Err(CoreError::CachePath);
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            copy_tree_inner(&item.path(), &destination.join(item.file_name()), progress)?;
        }
    } else if metadata.is_file() {
        copy_file(source, destination)?;
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

pub(super) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    let before = fs::symlink_metadata(source)?;
    if !before.is_file() {
        return Err(CoreError::CachePath);
    }
    if destination.exists() {
        let expected = digest(source)?;
        if !fs::symlink_metadata(destination)?.is_file() || digest(destination)? != expected {
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
        return Ok(());
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
            || digest(&temporary)? != expected.finalize().to_vec()
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
    result
}
