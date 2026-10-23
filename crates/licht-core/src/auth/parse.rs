use crate::auth::types::{
    AccountProfile, EntitlementsResponse, MinecraftLoginResponse, MinecraftProfileResponse,
    TokenErrorBody, TokenSuccess, XboxAuthResponse, XstsErrorBody,
};
use crate::{CoreError, Result};

pub fn parse_token_response(status: u16, body: &str) -> Result<TokenSuccess> {
    if status == 200 {
        let token: TokenSuccess = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
        if token.access_token.is_empty() {
            return Err(CoreError::AuthToken);
        }
        return Ok(token);
    }
    let error: TokenErrorBody = serde_json::from_str(body).map_err(CoreError::AuthJson)?;
    match error.error.as_str() {
        "access_denied" | "authorization_declined" => Err(CoreError::AuthDeclined),
        "expired_token" | "code_expired" => Err(CoreError::AuthBrowserTimeout),
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
        skin_url: raw
            .skins
            .into_iter()
            .find(|skin| skin.state == "ACTIVE")
            .map(|skin| validate_skin_url(&skin.url))
            .transpose()?,
    })
}

pub(super) fn validate_skin_url(value: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(value).map_err(|_| CoreError::AuthProfileMissing)?;
    let hash = url
        .path()
        .strip_prefix("/texture/")
        .ok_or(CoreError::AuthProfileMissing)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str() != Some("textures.minecraft.net")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || hash.len() != 64
        || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(CoreError::AuthProfileMissing);
    }
    url.set_scheme("https")
        .map_err(|_| CoreError::AuthProfileMissing)?;
    Ok(url.to_string())
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
    fn declined_token_maps_to_typed_error() {
        let body = r#"{"error":"access_denied","error_description":"user cancelled"}"#;
        let error = parse_token_response(400, body).expect_err("declined");
        assert!(matches!(error, CoreError::AuthDeclined));
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

    #[test]
    fn minecraft_profile_exposes_only_the_active_skin_over_https() {
        let hash = "a".repeat(64);
        let body = serde_json::json!({
            "id": "0123456789abcdef0123456789abcdef", "name": "Player",
            "skins": [
                {"state": "INACTIVE", "url": "invalid-unused-value"},
                {"state": "ACTIVE", "url": format!("http://textures.minecraft.net/texture/{hash}")}
            ]
        })
        .to_string();
        let profile = parse_minecraft_profile(200, &body).expect("public profile");
        assert_eq!(
            profile.skin_url,
            Some(format!("https://textures.minecraft.net/texture/{hash}"))
        );
        let saved = serde_json::to_string(&profile).expect("save public profile");
        assert_eq!(
            serde_json::from_str::<AccountProfile>(&saved).expect("reload"),
            profile
        );
    }

    #[test]
    fn old_saved_profiles_and_profiles_without_skins_remain_usable() {
        let saved = r#"{"uuid":"0123456789abcdef0123456789abcdef","name":"Player"}"#;
        let old: AccountProfile = serde_json::from_str(saved).expect("old profile");
        assert!(old.skin_url.is_none());
        let profile = parse_minecraft_profile(
            200,
            r#"{"id":"0123456789abcdef0123456789abcdef","name":"Player"}"#,
        )
        .expect("profile without skins");
        assert_eq!(profile, old);
        assert!(
            serde_json::to_value(profile)
                .expect("public JSON")
                .get("skinUrl")
                .is_none()
        );
    }

    #[test]
    fn active_skin_urls_must_be_public_official_texture_urls() {
        let hash = "a".repeat(64);
        for url in [
            format!("https://example.com/texture/{hash}"),
            format!("https://textures.minecraft.net.evil.test/texture/{hash}"),
            format!("https://user:password@textures.minecraft.net/texture/{hash}"),
            format!("https://textures.minecraft.net:8443/texture/{hash}"),
            format!("https://textures.minecraft.net/texture/{hash}?token=fixture"),
            format!("https://textures.minecraft.net/texture/{hash}#fragment"),
            "https://textures.minecraft.net/texture/not-a-hash".into(),
            "file:///texture/fixture".into(),
        ] {
            let body = serde_json::json!({"id":"0123456789abcdef0123456789abcdef", "name":"Player", "skins":[{"state":"ACTIVE", "url":url}]}).to_string();
            assert!(matches!(
                parse_minecraft_profile(200, &body),
                Err(CoreError::AuthProfileMissing)
            ));
        }
    }
}
