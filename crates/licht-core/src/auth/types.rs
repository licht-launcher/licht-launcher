use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Public Minecraft profile (never includes tokens).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub uuid: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub skin_url: Option<String>,
}

/// Full session kept in memory / keyring — not exported to TypeScript.
#[derive(Debug, Clone)]
pub struct MicrosoftSession {
    pub profile: AccountProfile,
    pub msa_access_token: String,
    pub msa_refresh_token: Option<String>,
    pub minecraft_token: String,
    pub minecraft_expires_at_unix: u64,
    pub xuid: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenSuccess {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[allow(dead_code)]
    pub expires_in: Option<u64>,
    #[allow(dead_code)]
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenErrorBody {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct XboxAuthResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
pub(crate) struct XboxDisplayClaims {
    pub xui: Vec<XboxXui>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct XboxXui {
    pub uhs: String,
    #[serde(default)]
    pub xid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct XstsErrorBody {
    #[serde(rename = "XErr")]
    pub xerr: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftLoginResponse {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<MinecraftSkin>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftSkin {
    pub state: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EntitlementsResponse {
    #[serde(default)]
    pub items: Vec<EntitlementItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EntitlementItem {
    pub name: String,
}
