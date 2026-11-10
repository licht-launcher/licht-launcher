use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ModpackSource {
    Modrinth,
    Curseforge,
}

impl ModpackSource {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "modrinth" => Some(Self::Modrinth),
            "curseforge" => Some(Self::Curseforge),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModpackListItem {
    pub id: String,
    pub source: ModpackSource,
    pub name: String,
    pub description: String,
    #[ts(type = "number")]
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub author: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModpackVersionItem {
    pub id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

/// One page of the merged Modrinth + CurseForge modpack search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModpackSearchPage {
    pub items: Vec<ModpackListItem>,
    /// 1-based page number this result belongs to.
    pub page: u32,
    /// Number of pages the largest source can serve; `0` when nothing matched.
    pub page_count: u32,
}

/// One modpack's store page data. `description_html` is already sanitized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModpackDetails {
    pub id: String,
    pub source: ModpackSource,
    pub name: String,
    pub summary: String,
    pub author: String,
    #[ts(type = "number")]
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    /// RFC 3339 timestamp of the last update, as reported by the source.
    pub updated_at: Option<String>,
    /// HTTPS image URLs, in the order the source lists them.
    pub gallery: Vec<String>,
    pub project_url: Option<String>,
    pub description_html: String,
}

/// One source's slice of a search page, with that source's total hit count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourcePage {
    pub(crate) items: Vec<ModpackListItem>,
    pub(crate) total: u64,
}
