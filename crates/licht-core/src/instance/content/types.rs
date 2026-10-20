use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum InstanceContentKind {
    Mods,
    ResourcePacks,
    ShaderPacks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceContentEntry {
    pub name: String,
    pub kind: InstanceContentKind,
    pub enabled: bool,
    #[ts(type = "number")]
    pub size: u64,
    pub title: String,
    pub version: Option<String>,
    pub icon_url: Option<String>,
}

/// The first event contains the local listing; later events contain enriched batches.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstanceContentUpdate {
    pub request_id: String,
    pub instance_folder: String,
    pub entries: Vec<InstanceContentEntry>,
}

impl InstanceContentKind {
    pub(super) fn sort_key(self) -> u8 {
        match self {
            Self::Mods => 0,
            Self::ResourcePacks => 1,
            Self::ShaderPacks => 2,
        }
    }
}
