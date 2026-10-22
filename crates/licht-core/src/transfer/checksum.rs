use std::io::Read;
use std::path::Path;

use md5::{Digest as Md5Digest, Md5};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

use crate::{CoreError, Result, hashes::hex_encode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadChecksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    Sha1AndSha512 { sha1: String, sha512: String },
    Md5(String),
    Unverified,
}

impl DownloadChecksum {
    pub(crate) fn legacy_sha1(value: &str) -> Self {
        if value.is_empty() {
            Self::Unverified
        } else {
            Self::Sha1(value.to_string())
        }
    }

    pub(crate) fn is_verified(&self) -> bool {
        !matches!(self, Self::Unverified)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Sha1(value) => validate_hex(value, 40),
            Self::Sha256(value) => validate_hex(value, 64),
            Self::Sha512(value) => validate_hex(value, 128),
            Self::Md5(value) => validate_hex(value, 32),
            Self::Sha1AndSha512 { sha1, sha512 } => {
                validate_hex(sha1, 40)?;
                validate_hex(sha512, 128)
            }
            Self::Unverified => Ok(()),
        }
    }

    pub(crate) fn hasher(&self) -> TransferHasher {
        match self {
            Self::Sha256(_) => TransferHasher::Sha256(Sha256::new()),
            Self::Sha512(_) => TransferHasher::Sha512(Sha512::new()),
            Self::Sha1AndSha512 { .. } => TransferHasher::Sha1AndSha512 {
                sha1: Sha1::new(),
                sha512: Sha512::new(),
            },
            Self::Md5(_) => TransferHasher::Md5(Md5::new()),
            _ => TransferHasher::Sha1(Sha1::new()),
        }
    }

    pub(crate) fn matches(&self, actual: &TransferDigest) -> bool {
        match (self, actual) {
            (
                Self::Sha1(expected)
                | Self::Sha256(expected)
                | Self::Sha512(expected)
                | Self::Md5(expected),
                TransferDigest::Single(actual),
            ) => expected.eq_ignore_ascii_case(actual),
            (
                Self::Sha1AndSha512 { sha1, sha512 },
                TransferDigest::Sha1AndSha512 {
                    sha1: actual_sha1,
                    sha512: actual_sha512,
                },
            ) => {
                sha1.eq_ignore_ascii_case(actual_sha1) && sha512.eq_ignore_ascii_case(actual_sha512)
            }
            _ => false,
        }
    }

    pub(crate) fn matches_file(&self, path: &Path) -> Result<bool> {
        if !self.is_verified() {
            return Ok(false);
        }
        let mut file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let mut hasher = self.hasher();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(self.matches(&hasher.finish()))
    }

    pub(crate) fn mismatch(&self, actual: TransferDigest) -> CoreError {
        match (self, actual) {
            (Self::Sha1(expected), TransferDigest::Single(actual)) => CoreError::Sha1Mismatch {
                expected: expected.clone(),
                actual,
            },
            (Self::Sha256(expected), TransferDigest::Single(actual)) => CoreError::Sha256Mismatch {
                expected: expected.clone(),
                actual,
            },
            (Self::Sha512(expected), TransferDigest::Single(actual)) => CoreError::Sha512Mismatch {
                expected: expected.clone(),
                actual,
            },
            (Self::Md5(expected), TransferDigest::Single(actual)) => CoreError::Md5Mismatch {
                expected: expected.clone(),
                actual,
            },
            (
                Self::Sha1AndSha512 { sha1: expected, .. },
                TransferDigest::Sha1AndSha512 { sha1, .. },
            ) if !expected.eq_ignore_ascii_case(&sha1) => CoreError::Sha1Mismatch {
                expected: expected.clone(),
                actual: sha1,
            },
            (
                Self::Sha1AndSha512 {
                    sha512: expected, ..
                },
                TransferDigest::Sha1AndSha512 { sha512, .. },
            ) => CoreError::Sha512Mismatch {
                expected: expected.clone(),
                actual: sha512,
            },
            (Self::Unverified, _) => CoreError::DownloadResponse {
                detail: "checksum is unavailable",
            },
            _ => CoreError::DownloadResponse {
                detail: "checksum result does not match its algorithm",
            },
        }
    }
}

fn validate_hex(value: &str, length: usize) -> Result<()> {
    if value.len() != length || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CoreError::DownloadResponse {
            detail: "invalid published checksum",
        });
    }
    Ok(())
}

pub(crate) enum TransferDigest {
    Single(String),
    Sha1AndSha512 { sha1: String, sha512: String },
}

#[derive(Clone)]
pub(crate) enum TransferHasher {
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
    Sha1AndSha512 { sha1: Sha1, sha512: Sha512 },
    Md5(Md5),
}

impl TransferHasher {
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(hasher) => hasher.update(bytes),
            Self::Sha256(hasher) => hasher.update(bytes),
            Self::Sha512(hasher) => hasher.update(bytes),
            Self::Sha1AndSha512 { sha1, sha512 } => {
                sha1.update(bytes);
                sha512.update(bytes);
            }
            Self::Md5(hasher) => hasher.update(bytes),
        }
    }

    pub(crate) fn finish(self) -> TransferDigest {
        match self {
            Self::Sha1(hasher) => TransferDigest::Single(hex_encode(hasher.finalize().as_slice())),
            Self::Sha256(hasher) => {
                TransferDigest::Single(hex_encode(hasher.finalize().as_slice()))
            }
            Self::Sha512(hasher) => {
                TransferDigest::Single(hex_encode(hasher.finalize().as_slice()))
            }
            Self::Md5(hasher) => TransferDigest::Single(hex_encode(hasher.finalize().as_slice())),
            Self::Sha1AndSha512 { sha1, sha512 } => TransferDigest::Sha1AndSha512 {
                sha1: hex_encode(sha1.finalize().as_slice()),
                sha512: hex_encode(sha512.finalize().as_slice()),
            },
        }
    }
}
