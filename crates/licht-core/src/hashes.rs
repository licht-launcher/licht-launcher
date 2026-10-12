//! Shared hex encoding and SHA-1 file checks.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::Result;

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

pub(crate) fn sha1_hex(bytes: &[u8]) -> String {
    hex_encode(Sha1::digest(bytes).as_slice())
}

/// `true` when the file exists and its SHA-1 matches. An empty expected hash never matches
/// (existence alone is not verification).
pub(crate) fn file_matches_sha1(path: &Path, expected_sha1: &str) -> Result<bool> {
    if expected_sha1.is_empty() {
        return Ok(false);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_lowercase_hex() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xab, 0xff]), "000fabff");
    }

    #[test]
    fn an_empty_sha1_does_not_treat_an_existing_file_as_verified() {
        let directory =
            std::env::temp_dir().join(format!("licht-hashes-empty-sha-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("file.bin");
        std::fs::write(&path, b"hello").unwrap();
        assert!(!file_matches_sha1(&path, "").expect("match"));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn sha1_hex_matches_known_digest() {
        assert_eq!(
            sha1_hex(b"hello world"),
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
        );
    }
}
