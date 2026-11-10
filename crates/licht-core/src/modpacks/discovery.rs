//! Store-style discovery: the merged modpack search and one pack's details.

use reqwest::Client;

use super::types::{ModpackDetails, ModpackListItem, ModpackSearchPage, ModpackSource, SourcePage};
use super::{curseforge, modrinth};
use crate::{CoreError, Result};

/// Results each source contributes to one page of the merged search.
const PAGE_SIZE_PER_SOURCE: u32 = 10;

/// Searches Modrinth always; CurseForge when `LICHT_CURSEFORGE_API_KEY` is set.
///
/// Both sources are asked for the same 1-based `page`, and that page is sorted
/// by downloads. The order is exact within a page, not across pages.
pub async fn search_modpacks(client: &Client, query: &str, page: u32) -> Result<ModpackSearchPage> {
    if page == 0 {
        return Err(CoreError::ModpackMeta {
            detail: "search_page_zero".into(),
        });
    }
    let offset = u64::from(page - 1) * u64::from(PAGE_SIZE_PER_SOURCE);
    let curseforge_search = async {
        match curseforge::search_curseforge(client, query, offset, PAGE_SIZE_PER_SOURCE).await {
            Ok(found) => Ok(Some(found)),
            Err(CoreError::CurseforgeKeyMissing) => Ok(None),
            Err(error) => Err(error),
        }
    };
    let (modrinth_page, curseforge_page) = tokio::join!(
        modrinth::search_modrinth(client, query, offset, PAGE_SIZE_PER_SOURCE),
        curseforge_search,
    );
    let mut sources = vec![modrinth_page?];
    sources.extend(curseforge_page?);
    Ok(merge_search_pages(page, sources))
}

fn merge_search_pages(page: u32, sources: Vec<SourcePage>) -> ModpackSearchPage {
    let largest = sources.iter().map(|source| source.total).max().unwrap_or(0);
    let page_count = largest.div_ceil(u64::from(PAGE_SIZE_PER_SOURCE));
    let mut items: Vec<ModpackListItem> = sources
        .into_iter()
        .flat_map(|source| source.items)
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.downloads));
    ModpackSearchPage {
        items,
        page,
        page_count: u32::try_from(page_count).unwrap_or(u32::MAX),
    }
}

/// Store page data for one modpack, with its description already sanitized.
pub async fn get_modpack(
    client: &Client,
    source: ModpackSource,
    project_id: &str,
) -> Result<ModpackDetails> {
    match source {
        ModpackSource::Modrinth => modrinth::get_modrinth_project(client, project_id).await,
        ModpackSource::Curseforge => curseforge::get_curseforge_mod(client, project_id).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, source: ModpackSource, downloads: u64) -> ModpackListItem {
        ModpackListItem {
            id: name.into(),
            source,
            name: name.into(),
            description: String::new(),
            downloads,
            icon_url: None,
            author: String::new(),
        }
    }

    #[test]
    fn merges_both_sources_sorted_by_downloads() {
        let page = merge_search_pages(
            2,
            vec![
                SourcePage {
                    items: vec![item("a", ModpackSource::Modrinth, 5)],
                    total: 25,
                },
                SourcePage {
                    items: vec![item("b", ModpackSource::Curseforge, 9)],
                    total: 12,
                },
            ],
        );
        let names: Vec<&str> = page.items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, vec!["b", "a"]);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_count, 3);
    }

    #[test]
    fn no_hits_means_no_pages() {
        let page = merge_search_pages(
            1,
            vec![SourcePage {
                items: vec![],
                total: 0,
            }],
        );
        assert!(page.items.is_empty());
        assert_eq!(page.page_count, 0);
    }

    #[tokio::test]
    async fn page_zero_is_rejected_before_any_request() {
        let client = Client::new();
        assert!(search_modpacks(&client, "", 0).await.is_err());
    }
}
