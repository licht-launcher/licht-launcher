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
    /// RFC 3339 UTC timestamp of the last update, as reported by the source.
    pub updated_at: Option<String>,
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

/// Mod loaders a modpack search can be narrowed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ModpackLoader {
    Fabric,
    Forge,
    Neoforge,
    Quilt,
}

/// Licht's shared category list; each source maps it to its own categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ModpackCategory {
    Adventure,
    Challenging,
    Combat,
    KitchenSink,
    Lightweight,
    Magic,
    Multiplayer,
    Optimization,
    Quests,
    Technology,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ModpackSort {
    #[default]
    Downloads,
    Updated,
    Relevance,
}

/// Optional narrowing of the merged modpack search. `None` means "any".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModpackSearchFilters {
    /// A Minecraft version id such as `1.20.1`.
    pub game_version: Option<String>,
    pub loader: Option<ModpackLoader>,
    pub category: Option<ModpackCategory>,
    /// Search only this source; `None` searches both.
    pub source: Option<ModpackSource>,
    pub sort: ModpackSort,
}

impl ModpackSearchFilters {
    /// The game version without surrounding spaces, or `None` when blank.
    pub(crate) fn game_version(&self) -> Option<&str> {
        self.game_version
            .as_deref()
            .map(str::trim)
            .filter(|version| !version.is_empty())
    }
}
