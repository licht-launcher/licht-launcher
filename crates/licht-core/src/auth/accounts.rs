use crate::auth::browser::browser_login_msa;
use crate::auth::flow::{
    complete_microsoft_login, minecraft_token_needs_refresh, refresh_microsoft_session,
};
use crate::auth::keyring_store::{
    delete_account, list_stored_profiles, load_account, load_active_account,
    set_active_account_uuid, store_account,
};
use crate::auth::types::{AccountProfile, MicrosoftSession};
use crate::{DatabaseClient, Result, SharedCache};

pub async fn list_accounts(database: &DatabaseClient) -> Result<Vec<AccountProfile>> {
    let uuids = database.remembered_account_uuids().await?;
    tokio::task::spawn_blocking(move || list_stored_profiles(&uuids))
        .await
        .map_err(|_| crate::CoreError::AuthKeyring)?
}

pub fn get_active_profile(_cache: &SharedCache) -> Result<Option<AccountProfile>> {
    Ok(load_active_account()?.map(|session| session.profile))
}

/// Browser authorization-code + PKCE login. `open_url` must open the system browser.
pub async fn login_with_browser<F>(
    client: &reqwest::Client,
    database: &DatabaseClient,
    open_url: F,
) -> Result<AccountProfile>
where
    F: FnOnce(&str) -> std::result::Result<(), String>,
{
    let msa = browser_login_msa(client, open_url).await?;
    let session = complete_microsoft_login(client, msa).await?;
    store_account(&session)?;
    database
        .remember_account_uuid(&session.profile.uuid, true)
        .await?;
    Ok(session.profile)
}

pub async fn set_active_account(database: &DatabaseClient, uuid: &str) -> Result<AccountProfile> {
    let session = load_account(uuid)?.ok_or(crate::CoreError::AuthNotSignedIn)?;
    set_active_account_uuid(uuid)?;
    database.remember_account_uuid(uuid, true).await?;
    Ok(session.profile)
}

pub async fn logout_account(database: &DatabaseClient, uuid: &str) -> Result<()> {
    delete_account(uuid)?;
    database.remember_account_uuid(uuid, false).await?;
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
