use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LoaderPlatform {
    Vanilla,
    Fabric,
    NeoForge,
    Forge,
    Quilt,
}

impl LoaderPlatform {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "vanilla" => Some(Self::Vanilla),
            "fabric" => Some(Self::Fabric),
            "neoforge" => Some(Self::NeoForge),
            "forge" => Some(Self::Forge),
            "quilt" => Some(Self::Quilt),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LoaderChannel {
    Stable,
    Latest,
    Manual,
}

impl LoaderChannel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stable" => Some(Self::Stable),
            "latest" => Some(Self::Latest),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}
