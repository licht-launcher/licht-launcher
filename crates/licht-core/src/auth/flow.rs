use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde_json::json;

use crate::auth::client_id::msa_client_id;
use crate::auth::endpoints::{
    MC_ENTITLEMENTS_URL, MC_LOGIN_URL, MC_PROFILE_URL, MSA_DEVICE_CODE_URL, MSA_SCOPE,
    MSA_TOKEN_URL, XBL_USER_AUTH_URL, XSTS_AUTHORIZE_URL,
};
use crate::auth::parse::{
    TokenPoll, parse_device_code_response, parse_minecraft_login, parse_minecraft_profile,
    parse_owns_minecraft, parse_token_poll, parse_xbox_auth, parse_xsts,
};
use crate::auth::types::{AccountProfile, DeviceCodeChallenge, MicrosoftSession, TokenSuccess};
use crate::{CoreError, Result};

/// Starts the device-code flow and returns the challenge for the UI.
pub async fn begin_device_code_login(client: &Client) -> Result<DeviceCodeChallenge> {
    let client_id = msa_client_id()?;
    let response = client
        .post(MSA_DEVICE_CODE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[("client_id", client_id.as_str()), ("scope", MSA_SCOPE)])
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::AuthOAuth {
            error: format!("device_code_http_{status}"),
            description: body,
        });
    }
    parse_device_code_response(&body)
}

/// One poll of the device-code token endpoint.
pub async fn poll_device_code_once(
    client: &Client,
    device_code: &str,
) -> Result<Option<TokenSuccess>> {
    let client_id = msa_client_id()?;
    let response = client
        .post(MSA_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id.as_str()),
            ("device_code", device_code),
        ])
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    match parse_token_poll(status, &body)? {
        TokenPoll::Pending => Ok(None),
        TokenPoll::Success(token) => Ok(Some(token)),
    }
}

/// Xbox Live → XSTS → Minecraft login → ownership → profile.
pub async fn complete_microsoft_login(
    client: &Client,
    msa: TokenSuccess,
) -> Result<MicrosoftSession> {
    let (xbl_token, user_hash, _) = xbox_user_authenticate(client, &msa.access_token).await?;
    let (xsts_token, xsts_hash, xuid) = xsts_authorize(client, &xbl_token).await?;
    let hash = if xsts_hash.is_empty() {
        user_hash
    } else {
        xsts_hash
    };
    let (minecraft_token, expires_in) = minecraft_login(client, &hash, &xsts_token).await?;
    if !owns_minecraft(client, &minecraft_token).await? {
        return Err(CoreError::AuthNoOwnership);
    }
    let profile = minecraft_profile(client, &minecraft_token).await?;
    Ok(MicrosoftSession {
        profile,
        msa_access_token: msa.access_token,
        msa_refresh_token: msa.refresh_token,
        minecraft_token,
        minecraft_expires_at_unix: unix_now().saturating_add(expires_in),
        xuid,
    })
}

/// Refresh MSA access token, then rebuild the Minecraft session.
pub async fn refresh_microsoft_session(
    client: &Client,
    refresh_token: &str,
) -> Result<MicrosoftSession> {
    let client_id = msa_client_id()?;
    let response = client
        .post(MSA_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token),
            ("scope", MSA_SCOPE),
        ])
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    let TokenPoll::Success(msa) = parse_token_poll(status, &body)? else {
        return Err(CoreError::AuthToken);
    };
    complete_microsoft_login(client, msa).await
}

async fn xbox_user_authenticate(
    client: &Client,
    msa_token: &str,
) -> Result<(String, String, String)> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={msa_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });
    let response = client
        .post(XBL_USER_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::AuthXbox);
    }
    parse_xbox_auth(&text)
}

async fn xsts_authorize(client: &Client, xbl_token: &str) -> Result<(String, String, String)> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });
    let response = client
        .post(XSTS_AUTHORIZE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    parse_xsts(status, &text)
}

async fn minecraft_login(
    client: &Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<(String, u64)> {
    let body = json!({
        "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")
    });
    let response = client
        .post(MC_LOGIN_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    parse_minecraft_login(status, &text)
}

async fn owns_minecraft(client: &Client, minecraft_token: &str) -> Result<bool> {
    let response = client
        .get(MC_ENTITLEMENTS_URL)
        .bearer_auth(minecraft_token)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    parse_owns_minecraft(status, &text)
}

async fn minecraft_profile(client: &Client, minecraft_token: &str) -> Result<AccountProfile> {
    let response = client
        .get(MC_PROFILE_URL)
        .bearer_auth(minecraft_token)
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    parse_minecraft_profile(status, &text)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// True when the Minecraft access token should be refreshed soon.
pub fn minecraft_token_needs_refresh(session: &MicrosoftSession, skew_seconds: u64) -> bool {
    let now = unix_now();
    session.minecraft_expires_at_unix <= now.saturating_add(skew_seconds)
}
