use serde::{Deserialize, Serialize};

use crate::auth::types::{AccountProfile, MicrosoftSession};
use crate::{CoreError, Result};

const SERVICE: &str = "app.licht.launcher";
const ACTIVE_ACCOUNT_KEY: &str = "active_account_uuid";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAccount {
    profile: AccountProfile,
    msa_refresh_token: Option<String>,
    minecraft_token: String,
    minecraft_expires_at_unix: u64,
    xuid: String,
}

fn entry(user: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, user).map_err(|_| CoreError::AuthKeyring)
}

/// Persists a Microsoft session in the OS keyring (secrets never logged).
pub fn store_account(session: &MicrosoftSession) -> Result<()> {
    let stored = StoredAccount {
        profile: session.profile.clone(),
        msa_refresh_token: session.msa_refresh_token.clone(),
        minecraft_token: session.minecraft_token.clone(),
        minecraft_expires_at_unix: session.minecraft_expires_at_unix,
        xuid: session.xuid.clone(),
    };
    let json = serde_json::to_string(&stored).map_err(CoreError::AuthJson)?;
    entry(&session.profile.uuid)?
        .set_password(&json)
        .map_err(|_| CoreError::AuthKeyring)?;
    set_active_account_uuid(&session.profile.uuid)?;
    Ok(())
}

pub fn load_account(uuid: &str) -> Result<Option<MicrosoftSession>> {
    let password = match entry(uuid)?.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(_) => return Err(CoreError::AuthKeyring),
    };
    let stored: StoredAccount = serde_json::from_str(&password).map_err(CoreError::AuthJson)?;
    Ok(Some(MicrosoftSession {
        profile: stored.profile,
        msa_access_token: String::new(),
        msa_refresh_token: stored.msa_refresh_token,
        minecraft_token: stored.minecraft_token,
        minecraft_expires_at_unix: stored.minecraft_expires_at_unix,
        xuid: stored.xuid,
    }))
}

pub fn delete_account(uuid: &str) -> Result<()> {
    match entry(uuid)?.delete_credential() {
        Ok(()) => {}
        Err(keyring::Error::NoEntry) => {}
        Err(_) => return Err(CoreError::AuthKeyring),
    }
    if active_account_uuid()?.as_deref() == Some(uuid) {
        clear_active_account_uuid()?;
    }
    Ok(())
}

pub fn set_active_account_uuid(uuid: &str) -> Result<()> {
    entry(ACTIVE_ACCOUNT_KEY)?
        .set_password(uuid)
        .map_err(|_| CoreError::AuthKeyring)
}

pub fn active_account_uuid() -> Result<Option<String>> {
    match entry(ACTIVE_ACCOUNT_KEY)?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(CoreError::AuthKeyring),
    }
}

pub fn clear_active_account_uuid() -> Result<()> {
    match entry(ACTIVE_ACCOUNT_KEY)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(CoreError::AuthKeyring),
    }
}

pub fn load_active_account() -> Result<Option<MicrosoftSession>> {
    let Some(uuid) = active_account_uuid()? else {
        return Ok(None);
    };
    load_account(&uuid)
}

/// Lists profiles known via the active pointer plus optional extras stored in DB meta.
pub fn list_stored_profiles(uuids: &[String]) -> Result<Vec<AccountProfile>> {
    let mut profiles = Vec::new();
    for uuid in uuids {
        if let Some(session) = load_account(uuid)? {
            profiles.push(session.profile);
        }
    }
    Ok(profiles)
}
