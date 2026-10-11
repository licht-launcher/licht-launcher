use crate::auth::types::{
    AccountProfile, DeviceCodeChallenge, DeviceCodeResponse, EntitlementsResponse,
    MinecraftLoginResponse, MinecraftProfileResponse, TokenErrorBody, TokenSuccess,
    XboxAuthResponse, XstsErrorBody,
};
use crate::{CoreError, Result};

pub fn parse_device_code_response(body: &str) -> Result<DeviceCodeChallenge> {
    let raw: DeviceCodeResponse = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    Ok(DeviceCodeChallenge {
        user_code: raw.user_code,
        verification_uri: raw
            .verification_uri_complete
            .unwrap_or(raw.verification_uri),
        device_code: raw.device_code,
        interval_seconds: raw.interval.max(1),
        expires_in_seconds: raw.expires_in,
    })
}

pub enum TokenPoll {
    Pending,
    Success(TokenSuccess),
}

pub fn parse_token_poll(status: u16, body: &str) -> Result<TokenPoll> {
    if status == 200 {
        let token: TokenSuccess = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
        if token.access_token.is_empty() {
            return Err(CoreError::AuthToken);
        }
        return Ok(TokenPoll::Success(token));
    }
    let error: TokenErrorBody = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    match error.error.as_str() {
        "authorization_pending" => Ok(TokenPoll::Pending),
        "slow_down" => Ok(TokenPoll::Pending),
        "expired_token" | "code_expired" => Err(CoreError::AuthDeviceCodeExpired),
        "authorization_declined" => Err(CoreError::AuthDeclined),
        other => Err(CoreError::AuthOAuth {
            error: other.to_string(),
            description: error.error_description.unwrap_or_default(),
        }),
    }
}

pub fn parse_xbox_auth(body: &str) -> Result<(String, String, String)> {
    let raw: XboxAuthResponse = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    let xui = raw
        .display_claims
        .xui
        .into_iter()
        .next()
        .ok_or(CoreError::AuthXbox)?;
    Ok((raw.token, xui.uhs, xui.xid.unwrap_or_default()))
}

pub fn parse_xsts(status: u16, body: &str) -> Result<(String, String, String)> {
    if status == 200 {
        return parse_xbox_auth(body);
    }
    if let Ok(error) = serde_json::from_str::<XstsErrorBody>(body) {
        return Err(map_xerr(error.xerr.unwrap_or(0)));
    }
    Err(CoreError::AuthXbox)
}

fn map_xerr(code: u64) -> CoreError {
    match code {
        2_148_916_227 => CoreError::AuthXboxBanned,
        2_148_916_233 => CoreError::AuthXboxNoAccount,
        2_148_916_235 => CoreError::AuthXboxUnavailable,
        2_148_916_236 | 2_148_916_237 => CoreError::AuthXboxAdultVerification,
        2_148_916_238 => CoreError::AuthXboxChild,
        _ => CoreError::AuthXbox,
    }
}

pub fn parse_minecraft_login(status: u16, body: &str) -> Result<(String, u64)> {
    if status == 403 {
        return Err(CoreError::AuthAppNotAllowlisted);
    }
    if !(200..300).contains(&status) {
        return Err(CoreError::AuthMinecraft);
    }
    let raw: MinecraftLoginResponse = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    if raw.access_token.is_empty() {
        return Err(CoreError::AuthMinecraft);
    }
    Ok((raw.access_token, raw.expires_in))
}

pub fn parse_minecraft_profile(status: u16, body: &str) -> Result<AccountProfile> {
    if status == 404 {
        return Err(CoreError::AuthProfileMissing);
    }
    if !(200..300).contains(&status) {
        return Err(CoreError::AuthMinecraft);
    }
    let raw: MinecraftProfileResponse = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    let uuid = normalize_uuid(&raw.id);
    if uuid.is_empty() || raw.name.is_empty() {
        return Err(CoreError::AuthProfileMissing);
    }
    Ok(AccountProfile {
        uuid,
        name: raw.name,
    })
}

pub fn parse_owns_minecraft(status: u16, body: &str) -> Result<bool> {
    if status == 403 {
        return Err(CoreError::AuthAppNotAllowlisted);
    }
    if !(200..300).contains(&status) {
        return Err(CoreError::AuthMinecraft);
    }
    let raw: EntitlementsResponse = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    Ok(raw.items.iter().any(|item| {
        matches!(
            item.name.as_str(),
            "product_minecraft"
                | "game_minecraft"
                | "product_game_pass_pc"
                | "product_game_pass_ultimate"
        )
    }))
}

fn normalize_uuid(id: &str) -> String {
    id.chars()
        .filter(|ch| *ch != '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_code_uses_complete_uri_when_present() {
        let body = r#"{
            "user_code":"ABCD1234",
            "device_code":"secret",
            "verification_uri":"https://microsoft.com/link",
            "verification_uri_complete":"https://microsoft.com/link?otc=ABCD1234",
            "expires_in":900,
            "interval":5
        }"#;
        let challenge = parse_device_code_response(body).expect("ok");
        assert_eq!(challenge.user_code, "ABCD1234");
        assert!(challenge.verification_uri.contains("otc="));
        assert_eq!(challenge.interval_seconds, 5);
    }

    #[test]
    fn pending_token_poll_is_not_an_error() {
        let body = r#"{"error":"authorization_pending","error_description":"wait"}"#;
        let poll = parse_token_poll(400, body).expect("pending");
        assert!(matches!(poll, TokenPoll::Pending));
    }

    #[test]
    fn child_xsts_maps_to_typed_error() {
        let body = r#"{"Identity":"0","XErr":2148916238,"Message":""}"#;
        let error = parse_xsts(401, body).expect_err("child");
        assert!(matches!(error, CoreError::AuthXboxChild));
    }

    #[test]
    fn minecraft_403_means_app_not_allowlisted() {
        let error = parse_minecraft_login(403, "{}").expect_err("403");
        assert!(matches!(error, CoreError::AuthAppNotAllowlisted));
    }

    #[test]
    fn entitlements_detect_game_pass() {
        let body = r#"{"items":[{"name":"product_game_pass_ultimate","signature":"x"}]}"#;
        assert!(parse_owns_minecraft(200, body).expect("ok"));
    }
}
