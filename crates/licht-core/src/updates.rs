use std::path::{Path, PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CoreError, Result, hashes::sha1_hex};

pub const APP_UPDATE_INTERVAL_SECONDS: u64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AppUpdatePhase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Cached,
    Ready,
    Installing,
    Failed,
    /// Another package manager (Flatpak) owns updates for this installation.
    ExternallyManaged,
    /// A portable build: no installer placed it, so it cannot replace itself.
    Portable,
    /// An all-users MSI install: silent updates cannot raise its UAC prompt.
    Msi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateState {
    pub phase: AppUpdatePhase,
    pub automatic_updates: bool,
    pub version: Option<String>,
    #[ts(type = "number")]
    pub downloaded_bytes: u64,
    #[ts(type = "number | null")]
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
}

impl AppUpdateState {
    pub fn new(automatic_updates: bool) -> Self {
        Self {
            phase: AppUpdatePhase::Idle,
            automatic_updates,
            version: None,
            downloaded_bytes: 0,
            total_bytes: None,
            error: None,
        }
    }
}

/// Flatpak creates this file inside every sandbox.
const FLATPAK_INFO_FILE: &str = "/.flatpak-info";

/// Inside a Flatpak sandbox the bundled binary cannot replace itself; Flatpak
/// installs new versions instead.
pub fn app_updates_managed_externally() -> bool {
    flatpak_sandbox(Path::new(FLATPAK_INFO_FILE))
}

fn flatpak_sandbox(info_file: &Path) -> bool {
    info_file.is_file()
}

pub fn app_update_can_check(phase: AppUpdatePhase) -> bool {
    matches!(
        phase,
        AppUpdatePhase::Idle
            | AppUpdatePhase::UpToDate
            | AppUpdatePhase::Available
            | AppUpdatePhase::Cached
            | AppUpdatePhase::Failed
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppUpdatePackage {
    pub version: String,
    pub target: String,
    pub signature: String,
    pub sha1: String,
    pub bytes: u64,
}

impl AppUpdatePackage {
    pub fn matches_release(&self, version: &str, signature: &str) -> bool {
        self.version == version && self.signature == signature
    }
}

pub struct AppUpdateCache {
    directory: PathBuf,
    current: Version,
    target: String,
    public_key: PublicKey,
    require_signed_version: bool,
}

fn update_error(message: &'static str) -> CoreError {
    CoreError::ApplicationUpdate(message)
}

fn version(value: &str) -> Result<Version> {
    Version::parse(value.trim_start_matches('v'))
        .map_err(|_| update_error("invalid update version"))
}

fn decoded_text(value: &str) -> Result<String> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| update_error("invalid signature encoding"))?;
    String::from_utf8(bytes).map_err(|_| update_error("invalid signature text"))
}

fn check_signed_version(comment: &str, announced: &str, required: bool) -> Result<()> {
    let signed = comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    match signed {
        Some(signed) if version(signed)? != version(announced)? => {
            Err(update_error("signature version does not match the update"))
        }
        None if required => Err(update_error("signature does not include an update version")),
        _ => Ok(()),
    }
}

fn read_file(path: &Path, maximum_size: Option<u64>) -> Result<Option<Vec<u8>>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(update_error("cache entry is not a regular file"));
    }
    if maximum_size.is_some_and(|maximum| metadata.len() > maximum) {
        return Err(update_error("cached update metadata is too large"));
    }
    Ok(Some(std::fs::read(path)?))
}

impl AppUpdateCache {
    pub fn new(
        directory: PathBuf,
        current: &str,
        target: &str,
        public_key: &str,
        require_signed_version: bool,
    ) -> Result<Self> {
        if target.is_empty()
            || target.len() > 128
            || !target.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '-' || character == '_'
            })
        {
            return Err(update_error("invalid update target"));
        }
        let public_key = PublicKey::decode(&decoded_text(public_key)?)
            .map_err(|_| update_error("invalid update public key"))?;
        Ok(Self {
            directory,
            current: version(current)?,
            target: target.to_string(),
            public_key,
            require_signed_version,
        })
    }

    fn verify(&self, announced: &str, signature: &str, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Err(update_error("empty update package"));
        }
        if version(announced)? <= self.current {
            return Err(update_error(
                "update is not newer than the installed version",
            ));
        }
        let signature = Signature::decode(&decoded_text(signature)?)
            .map_err(|_| update_error("invalid cached signature"))?;
        self.public_key
            .verify(bytes, &signature, true)
            .map_err(|_| update_error("cached update signature does not match"))?;
        check_signed_version(
            signature.trusted_comment(),
            announced,
            self.require_signed_version,
        )
    }

    pub fn load(&self) -> Result<Option<(AppUpdatePackage, Vec<u8>)>> {
        let manifest_path = self.directory.join("package.json");
        let Some(manifest) = read_file(&manifest_path, Some(64 * 1024))? else {
            return Ok(None);
        };
        let package: AppUpdatePackage = serde_json::from_slice(&manifest)
            .map_err(|_| update_error("invalid cached update metadata"))?;
        if package.target != self.target || version(&package.version)? <= self.current {
            self.clear()?;
            return Ok(None);
        }
        let Some(bytes) = read_file(&self.directory.join("package.bin"), None)? else {
            return Ok(None);
        };
        if package.bytes != bytes.len() as u64
            || !package.sha1.eq_ignore_ascii_case(&sha1_hex(&bytes))
        {
            return Err(update_error("cached update checksum does not match"));
        }
        self.verify(&package.version, &package.signature, &bytes)?;
        Ok(Some((package, bytes)))
    }

    pub fn store(
        &self,
        announced: &str,
        signature: &str,
        bytes: &[u8],
    ) -> Result<AppUpdatePackage> {
        self.verify(announced, signature, bytes)?;
        let package = AppUpdatePackage {
            version: version(announced)?.to_string(),
            target: self.target.clone(),
            signature: signature.to_string(),
            sha1: sha1_hex(bytes),
            bytes: bytes.len() as u64,
        };
        let manifest = serde_json::to_vec(&package)
            .map_err(|_| update_error("could not encode update metadata"))?;
        std::fs::create_dir_all(&self.directory)?;
        let body_path = self.directory.join("package.bin");
        let manifest_path = self.directory.join("package.json");
        std::fs::write(body_path.with_extension("bin.tmp"), bytes)?;
        std::fs::write(manifest_path.with_extension("json.tmp"), manifest)?;
        std::fs::rename(body_path.with_extension("bin.tmp"), body_path)?;
        std::fs::rename(manifest_path.with_extension("json.tmp"), manifest_path)?;
        Ok(package)
    }

    pub fn clear(&self) -> Result<()> {
        for name in [
            "package.json",
            "package.bin",
            "package.json.tmp",
            "package.bin.tmp",
        ] {
            remove_if_present(&self.directory.join(name))?;
        }
        Ok(())
    }
}

/// How long to keep retrying a file another process holds open, such as an antivirus
/// scanning a package that was just written.
const HELD_FILE_ATTEMPTS: u32 = 40;
const HELD_FILE_DELAY: std::time::Duration = std::time::Duration::from_millis(25);

/// Windows refuses to delete a file another process has open without sharing deletes.
const ERROR_SHARING_VIOLATION: i32 = 32;

fn remove_if_present(path: &Path) -> Result<()> {
    let mut attempt = 1;
    loop {
        match std::fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && error.raw_os_error() == Some(ERROR_SHARING_VIOLATION)
                    && attempt < HELD_FILE_ATTEMPTS =>
            {
                attempt += 1;
                std::thread::sleep(HELD_FILE_DELAY);
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use licht_testkit::temp::TempDir;

    use super::*;

    // Public verification vector from minisign-verify's MIT-licensed documentation.
    const KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1633700835\tfile:test\tprehashed\nwLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";

    /// The cache folder itself starts out missing, inside a folder removed on drop.
    fn cache(name: &str, current: &str, strict: bool) -> (TempDir, AppUpdateCache) {
        let temp = TempDir::new(&format!("update-cache-{name}"));
        let cache = AppUpdateCache::new(
            temp.path().join("updates"),
            current,
            "windows-x86_64",
            &STANDARD.encode(KEY),
            strict,
        )
        .expect("cache");
        (temp, cache)
    }

    #[test]
    fn verified_packages_survive_reopening_and_are_removed_after_installation() {
        let (_temp, cache) = cache("reopen", "1.0.0", false);
        let package = cache
            .store("1.1.0", &STANDARD.encode(SIGNATURE), b"test")
            .expect("store signed package");
        assert!(package.matches_release("1.1.0", &STANDARD.encode(SIGNATURE)));
        assert!(!package.matches_release("1.2.0", &STANDARD.encode(SIGNATURE)));
        assert!(!package.matches_release("1.1.0", "different signature"));
        let reopened = AppUpdateCache::new(
            cache.directory.clone(),
            "1.0.0",
            "windows-x86_64",
            &STANDARD.encode(KEY),
            false,
        )
        .expect("reopen");
        assert_eq!(
            reopened.load().expect("load"),
            Some((package, b"test".to_vec()))
        );
        let updated = AppUpdateCache::new(
            cache.directory.clone(),
            "1.1.0",
            "windows-x86_64",
            &STANDARD.encode(KEY),
            false,
        )
        .expect("updated version");
        assert!(updated.load().expect("obsolete package").is_none());
        assert!(!cache.directory.join("package.bin").exists());
    }

    /// Another process, such as an antivirus scanning a file just written, can hold it
    /// open for a moment without letting it be deleted.
    #[cfg(windows)]
    #[test]
    fn clearing_waits_for_a_briefly_held_package() {
        use std::os::windows::fs::OpenOptionsExt;

        let (_temp, cache) = cache("held", "1.0.0", false);
        cache
            .store("1.1.0", &STANDARD.encode(SIGNATURE), b"test")
            .expect("store signed package");
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(cache.directory.join("package.bin"))
            .expect("hold the package");
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            drop(held);
        });

        cache.clear().expect("clear once the package is released");
        release.join().expect("the holder should finish");

        assert!(!cache.directory.join("package.bin").exists());
    }

    #[test]
    fn invalid_or_modified_packages_never_become_ready() {
        let (_temp, cache) = cache("tampered", "1.0.0", false);
        assert!(
            cache
                .store("1.1.0", &STANDARD.encode(SIGNATURE), b"")
                .is_err()
        );
        assert!(
            cache
                .store("1.1.0", &STANDARD.encode(SIGNATURE), b"Test")
                .is_err()
        );
        assert!(!cache.directory.exists());
        cache
            .store("1.1.0", &STANDARD.encode(SIGNATURE), b"test")
            .expect("valid package");
        std::fs::write(cache.directory.join("package.bin"), b"Test").expect("tamper");
        assert!(cache.load().is_err());
        let metadata_path = cache.directory.join("package.json");
        let mut metadata: AppUpdatePackage =
            serde_json::from_slice(&std::fs::read(&metadata_path).expect("metadata"))
                .expect("json");
        metadata.sha1 = sha1_hex(b"Test");
        std::fs::write(&metadata_path, serde_json::to_vec(&metadata).expect("json"))
            .expect("changed metadata");
        assert!(cache.load().is_err());
        cache.clear().expect("clear");
        assert!(cache.load().expect("absent").is_none());
    }

    #[test]
    fn signature_version_policy_matches_the_updater_configuration() {
        let (_temp, cache) = cache("strict", "1.0.0", true);
        assert!(
            cache
                .store("1.1.0", &STANDARD.encode(SIGNATURE), b"test")
                .is_err()
        );
        assert!(check_signed_version("timestamp:1\tversion:v1.2.3", "1.2.3", true).is_ok());
        assert!(check_signed_version("timestamp:1\tversion:1.2.2", "1.2.3", false).is_err());
        assert!(check_signed_version("timestamp:1", "1.2.3", false).is_ok());
    }

    #[test]
    fn only_inactive_update_states_allow_background_checks() {
        assert!(app_update_can_check(AppUpdatePhase::Idle));
        assert!(app_update_can_check(AppUpdatePhase::Cached));
        for phase in [
            AppUpdatePhase::Checking,
            AppUpdatePhase::Downloading,
            AppUpdatePhase::Verifying,
            AppUpdatePhase::Ready,
            AppUpdatePhase::Installing,
            AppUpdatePhase::ExternallyManaged,
            AppUpdatePhase::Portable,
            AppUpdatePhase::Msi,
        ] {
            assert!(!app_update_can_check(phase));
        }
    }

    #[test]
    fn detects_a_flatpak_sandbox_from_its_info_file() {
        let temp = TempDir::new("flatpak-info");
        let directory = temp.path().to_path_buf();
        let info_file = directory.join(".flatpak-info");
        assert!(!flatpak_sandbox(&info_file));
        std::fs::write(&info_file, "[Application]\nname=app.licht.launcher\n").expect("info");
        assert!(flatpak_sandbox(&info_file));
    }

    #[test]
    fn a_directory_at_the_flatpak_info_path_is_not_a_sandbox() {
        let temp = TempDir::new("flatpak-dir");
        let directory = temp.path().to_path_buf();
        assert!(!flatpak_sandbox(&directory));
    }
}
