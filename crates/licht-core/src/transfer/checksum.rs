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

    pub(crate) fn expected(&self) -> Option<&str> {
        match self {
            Self::Sha1(value) | Self::Sha256(value) | Self::Sha512(value) | Self::Md5(value) => {
                Some(value)
            }
            Self::Unverified => None,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let length = match self {
            Self::Sha1(_) => 40,
            Self::Sha256(_) => 64,
            Self::Sha512(_) => 128,
            Self::Md5(_) => 32,
            Self::Unverified => return Ok(()),
        };
        if self.expected().is_none_or(|value| {
            value.len() != length || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(CoreError::DownloadResponse {
                detail: "invalid published checksum",
            });
        }
        Ok(())
    }

    pub(crate) fn hasher(&self) -> TransferHasher {
        match self {
            Self::Sha256(_) => TransferHasher::Sha256(Sha256::new()),
            Self::Sha512(_) => TransferHasher::Sha512(Sha512::new()),
            Self::Md5(_) => TransferHasher::Md5(Md5::new()),
            _ => TransferHasher::Sha1(Sha1::new()),
        }
    }

    pub(crate) fn matches(&self, actual: &str) -> bool {
        self.expected()
            .is_some_and(|expected| expected.eq_ignore_ascii_case(actual))
    }

    pub(crate) fn matches_file(&self, path: &Path) -> Result<bool> {
        if self.expected().is_none() {
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

    pub(crate) fn mismatch(&self, actual: String) -> CoreError {
        match self {
            Self::Sha1(expected) => CoreError::Sha1Mismatch {
                expected: expected.clone(),
                actual,
            },
            Self::Sha256(expected) => CoreError::Sha256Mismatch {
                expected: expected.clone(),
                actual,
            },
            Self::Sha512(expected) => CoreError::Sha512Mismatch {
                expected: expected.clone(),
                actual,
            },
            Self::Md5(expected) => CoreError::Md5Mismatch {
                expected: expected.clone(),
                actual,
            },
            Self::Unverified => CoreError::DownloadResponse {
                detail: "checksum is unavailable",
            },
        }
    }
}

#[derive(Clone)]
pub(crate) enum TransferHasher {
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
    Md5(Md5),
}

impl TransferHasher {
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(hasher) => hasher.update(bytes),
            Self::Sha256(hasher) => hasher.update(bytes),
            Self::Sha512(hasher) => hasher.update(bytes),
            Self::Md5(hasher) => hasher.update(bytes),
        }
    }

    pub(crate) fn finish(self) -> String {
        match self {
            Self::Sha1(hasher) => hex_encode(hasher.finalize().as_slice()),
            Self::Sha256(hasher) => hex_encode(hasher.finalize().as_slice()),
            Self::Sha512(hasher) => hex_encode(hasher.finalize().as_slice()),
            Self::Md5(hasher) => hex_encode(hasher.finalize().as_slice()),
        }
    }
}
