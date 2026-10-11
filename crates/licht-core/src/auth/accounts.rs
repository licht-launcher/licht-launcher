use crate::auth::browser::browser_login_msa;
use crate::auth::flow::{
    complete_microsoft_login, minecraft_token_needs_refresh, refresh_microsoft_session,
};
use crate::auth::keyring_store::{
    delete_account, list_stored_profiles, load_account, load_active_account,
    set_active_account_uuid, store_account,
};
use crate::auth::types::{AccountProfile, MicrosoftSession};
use crate::db::{get_meta, open_db, set_meta};
use crate::{Result, SharedCache};

const META_ACCOUNT_UUIDS: &str = "microsoft_account_uuids";

/// Account UUIDs remembered in SQLite (secrets stay in the keyring).
pub fn remembered_account_uuids(cache: &SharedCache) -> Result<Vec<String>> {
    let connection = open_db(cache)?;
    let Some(raw) = get_meta(&connection, META_ACCOUNT_UUIDS)? else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(raw
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect())
}

fn save_remembered_uuids(cache: &SharedCache, uuids: &[String]) -> Result<()> {
    let connection = open_db(cache)?;
    set_meta(&connection, META_ACCOUNT_UUIDS, &uuids.join(","))
}

fn remember_uuid(cache: &SharedCache, uuid: &str) -> Result<()> {
    let mut uuids = remembered_account_uuids(cache)?;
    if !uuids.iter().any(|existing| existing == uuid) {
        uuids.push(uuid.to_string());
        save_remembered_uuids(cache, &uuids)?;
    }
    Ok(())
}

fn forget_uuid(cache: &SharedCache, uuid: &str) -> Result<()> {
    let uuids = remembered_account_uuids(cache)?
        .into_iter()
        .filter(|existing| existing != uuid)
        .collect::<Vec<_>>();
    save_remembered_uuids(cache, &uuids)
}

pub fn list_accounts(cache: &SharedCache) -> Result<Vec<AccountProfile>> {
    let uuids = remembered_account_uuids(cache)?;
    list_stored_profiles(&uuids)
}

pub fn get_active_profile(_cache: &SharedCache) -> Result<Option<AccountProfile>> {
    Ok(load_active_account()?.map(|session| session.profile))
}

/// Browser authorization-code + PKCE login. `open_url` must open the system browser.
pub async fn login_with_browser<F>(
    client: &reqwest::Client,
    cache: &SharedCache,
    open_url: F,
) -> Result<AccountProfile>
where
    F: FnOnce(&str) -> std::result::Result<(), String>,
{
    let msa = browser_login_msa(client, open_url).await?;
    let session = complete_microsoft_login(client, msa).await?;
    store_account(&session)?;
    remember_uuid(cache, &session.profile.uuid)?;
    Ok(session.profile)
}

pub fn set_active_account(cache: &SharedCache, uuid: &str) -> Result<AccountProfile> {
    let session = load_account(uuid)?.ok_or(crate::CoreError::AuthNotSignedIn)?;
    set_active_account_uuid(uuid)?;
    remember_uuid(cache, uuid)?;
    Ok(session.profile)
}

pub fn logout_account(cache: &SharedCache, uuid: &str) -> Result<()> {
    delete_account(uuid)?;
    forget_uuid(cache, uuid)?;
    Ok(())
}

/// Session ready for launch (refreshes when the Minecraft token is near expiry).
pub async fn session_for_launch(
    client: &reqwest::Client,
    _cache: &SharedCache,
) -> Result<MicrosoftSession> {
    let session = load_active_account()?.ok_or(crate::CoreError::AuthNotSignedIn)?;
    if !minecraft_token_needs_refresh(&session, 120) {
        return Ok(session);
    }
    let Some(refresh) = session.msa_refresh_token.as_deref() else {
        return Err(crate::CoreError::AuthNotSignedIn);
    };
    let refreshed = refresh_microsoft_session(client, refresh).await?;
    store_account(&refreshed)?;
    Ok(refreshed)
}
