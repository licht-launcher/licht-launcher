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
            copy_tree(&item.path(), &destination.join(item.file_name()))?;
        }
    } else if metadata.is_file() {
        copy_file(source, destination)?;
    } else {
        return Err(CoreError::CachePath);
    }
    Ok(())
}

pub(super) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    let expected = digest(source)?;
    if destination.exists() {
        if !fs::symlink_metadata(destination)?.is_file() || digest(destination)? != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "launcher migration destination contains different data",
            )
            .into());
        }
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
        std::io::copy(&mut input, &mut output)?;
        output.flush()?;
        output.sync_all()?;
        drop(output);
        if digest(&temporary)? != expected || digest(source)? != expected {
            return Err(
                std::io::Error::other("launcher migration copy verification failed").into(),
            );
        }
        fs::rename(&temporary, destination)?;
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
