use std::fs;
use std::path::{Path, PathBuf};

use super::progress::StartupPreviewScenario;
use crate::{CoreError, Result, SharedCache};

fn path(name: &str) -> Result<PathBuf> {
    Ok(SharedCache::system()?.root().join("cache").join(name))
}

fn write(path: &Path, value: &serde_json::Value) -> Result<()> {
    fs::create_dir_all(path.parent().ok_or(CoreError::CachePath)?)?;
    fs::write(
        path,
        serde_json::to_vec(value).map_err(std::io::Error::other)?,
    )?;
    Ok(())
}

pub fn request_startup_update(version: &str) -> Result<()> {
    write(
        &path("update-request.json")?,
        &serde_json::json!({"fromVersion": version, "attempted": false}),
    )
}

pub fn mark_startup_update_attempt(version: &str) -> Result<()> {
    write(
        &path("update-request.json")?,
        &serde_json::json!({"fromVersion": version, "attempted": true}),
    )
}

pub fn clear_startup_update_request() -> Result<()> {
    match fs::remove_file(path("update-request.json")?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn read_startup_update_intent(version: &str) -> Result<(bool, bool)> {
    intent_at(&path("update-request.json")?, version)
}

fn intent_at(path: &Path, version: &str) -> Result<(bool, bool)> {
    if !path.exists() {
        return Ok((false, false));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path)?).map_err(std::io::Error::other)?;
    let from = value
        .get("fromVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| std::io::Error::other("invalid update request version"))?;
    let attempted = value
        .get("attempted")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| std::io::Error::other("invalid update request state"))?;
    if from != version {
        fs::remove_file(path)?;
        return Ok((false, false));
    }
    Ok((true, attempted))
}

pub struct StartupUpdatePolicy {
    pub check: bool,
    pub install: bool,
    pub blocked: bool,
}

pub fn startup_update_policy(
    check: bool,
    install: bool,
    manual: bool,
    attempted: bool,
) -> StartupUpdatePolicy {
    StartupUpdatePolicy {
        check: check || manual,
        install: install || manual,
        blocked: attempted,
    }
}

pub fn request_startup_preview(scenario: StartupPreviewScenario) -> Result<()> {
    if !cfg!(debug_assertions) {
        return Err(std::io::Error::other(
            "Startup tests are only available in development builds",
        )
        .into());
    }
    write(
        &path("startup-preview.json")?,
        &serde_json::to_value(scenario).map_err(std::io::Error::other)?,
    )
}

pub fn take_startup_preview() -> Result<Option<StartupPreviewScenario>> {
    if !cfg!(debug_assertions) {
        return Ok(None);
    }
    let path = path("startup-preview.json")?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    fs::remove_file(path)?;
    Ok(Some(
        serde_json::from_slice(&bytes).map_err(std::io::Error::other)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_update_overrides_preferences_while_automatic_startup_respects_them() {
        let disabled = startup_update_policy(false, false, false, false);
        assert!(!disabled.check && !disabled.install && !disabled.blocked);
        let notify = startup_update_policy(true, false, false, false);
        assert!(notify.check && !notify.install);
        let automatic = startup_update_policy(true, true, false, false);
        assert!(automatic.check && automatic.install);
        let manual = startup_update_policy(false, false, true, false);
        assert!(manual.check && manual.install);
        assert!(startup_update_policy(true, true, true, true).blocked);
    }

    #[test]
    fn pending_update_intent_detects_failed_restarts_and_is_consumed_by_a_new_version() {
        let root = std::env::temp_dir().join(format!(
            "licht-update-intent-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("directory");
        let path = root.join("request.json");
        assert_eq!(intent_at(&path, "1.0.0").expect("absent"), (false, false));
        fs::write(&path, br#"{"fromVersion":"1.0.0","attempted":false}"#).expect("request");
        assert_eq!(
            intent_at(&path, "1.0.0").expect("manual request"),
            (true, false)
        );
        fs::write(&path, br#"{"fromVersion":"1.0.0","attempted":true}"#).expect("attempted");
        assert_eq!(intent_at(&path, "1.0.0").expect("loop guard"), (true, true));
        assert_eq!(
            intent_at(&path, "1.1.0").expect("updated version"),
            (false, false)
        );
        assert!(!path.exists());
        fs::write(&path, br#"{"fromVersion":"1.0.0","attempted":"invalid"}"#)
            .expect("invalid input");
        assert!(intent_at(&path, "1.0.0").is_err());
    }
}
