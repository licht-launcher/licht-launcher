//! Store-style discovery: the merged modpack search and one pack's details.

use reqwest::Client;

use super::types::{
    ModpackDetails, ModpackListItem, ModpackSearchFilters, ModpackSearchPage, ModpackSort,
    ModpackSource, SourcePage,
};
use super::{curseforge, modrinth};
use crate::{CoreError, Result};

/// Results each source contributes to one page of the merged search.
const PAGE_SIZE_PER_SOURCE: u32 = 10;

/// Searches Modrinth and CurseForge (when `LICHT_CURSEFORGE_API_KEY` is set),
/// narrowed by `filters`.
///
/// Both sources are asked for the same 1-based `page`, and that page is ordered
/// by `filters.sort`. The order is exact within a page, not across pages.
pub async fn search_modpacks(
    client: &Client,
    query: &str,
    page: u32,
    filters: &ModpackSearchFilters,
) -> Result<ModpackSearchPage> {
    if page == 0 {
        return Err(CoreError::ModpackMeta {
            detail: "search_page_zero".into(),
        });
    }
    let offset = u64::from(page - 1) * u64::from(PAGE_SIZE_PER_SOURCE);
    let modrinth_search = async {
        if !searches_modrinth(filters) {
            return Ok(None);
        }
        modrinth::search_modrinth(client, query, offset, PAGE_SIZE_PER_SOURCE, filters)
            .await
            .map(Some)
    };
    let curseforge_search = async {
        if !searches_curseforge(filters) {
            return Ok(None);
        }
        let only_curseforge = filters.source == Some(ModpackSource::Curseforge);
        let result =
            curseforge::search_curseforge(client, query, offset, PAGE_SIZE_PER_SOURCE, filters)
                .await;
        curseforge_outcome(result, only_curseforge)
    };
    let (modrinth_page, curseforge_page) = tokio::join!(modrinth_search, curseforge_search);
    let sources: Vec<SourcePage> = modrinth_page?.into_iter().chain(curseforge_page?).collect();
    Ok(merge_search_pages(page, sources, filters.sort))
}

/// When both sources are searched, a CurseForge failure leaves its share out
/// so Modrinth results still show; a CurseForge-only search reports the error.
fn curseforge_outcome(
    result: Result<SourcePage>,
    only_curseforge: bool,
) -> Result<Option<SourcePage>> {
    match result {
        Ok(found) => Ok(Some(found)),
        Err(error) if only_curseforge => Err(error),
        // No key configured is the normal state for many users, not a failure.
        Err(CoreError::CurseforgeKeyMissing) => Ok(None),
        Err(error) => {
            tracing::warn!(%error, "CurseForge search failed; showing Modrinth results only");
            Ok(None)
        }
    }
}

fn searches_modrinth(filters: &ModpackSearchFilters) -> bool {
    filters.source != Some(ModpackSource::Curseforge)
}

/// CurseForge sits out when another source was picked or it has no match for
/// the chosen category.
fn searches_curseforge(filters: &ModpackSearchFilters) -> bool {
    filters.source != Some(ModpackSource::Modrinth)
        && filters.category.is_none_or(curseforge::serves_category)
}

fn merge_search_pages(page: u32, sources: Vec<SourcePage>, sort: ModpackSort) -> ModpackSearchPage {
    let largest = sources.iter().map(|source| source.total).max().unwrap_or(0);
    let page_count = largest.div_ceil(u64::from(PAGE_SIZE_PER_SOURCE));
    let lists: Vec<Vec<ModpackListItem>> = sources.into_iter().map(|source| source.items).collect();
    let items = match sort {
        ModpackSort::Downloads => {
            let mut items: Vec<ModpackListItem> = lists.into_iter().flatten().collect();
            items.sort_by_key(|item| std::cmp::Reverse(item.downloads));
            items
        }
        ModpackSort::Updated => {
            let mut items: Vec<ModpackListItem> = lists.into_iter().flatten().collect();
            // RFC 3339 UTC timestamps compare correctly as text (to the second);
            // items without a date go last because `None` sorts before `Some`.
            items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            items
        }
        // Relevance scores are not comparable across sources, so take turns
        // and keep each source's own order.
        ModpackSort::Relevance => interleave(lists),
    };
    ModpackSearchPage {
        items,
        page,
        page_count: u32::try_from(page_count).unwrap_or(u32::MAX),
    }
}

fn interleave(lists: Vec<Vec<ModpackListItem>>) -> Vec<ModpackListItem> {
    let mut iterators: Vec<_> = lists.into_iter().map(Vec::into_iter).collect();
    let mut items = Vec::new();
    loop {
        let before = items.len();
        for iterator in &mut iterators {
            items.extend(iterator.next());
        }
        if items.len() == before {
            return items;
        }
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
    use crate::modpacks::types::ModpackCategory;

    fn item(name: &str, source: ModpackSource, downloads: u64) -> ModpackListItem {
        ModpackListItem {
            id: name.into(),
            source,
            name: name.into(),
            description: String::new(),
            downloads,
            icon_url: None,
            author: String::new(),
            updated_at: None,
            loaders: Vec::new(),
            categories: Vec::new(),
        }
    }

    fn updated(name: &str, source: ModpackSource, at: Option<&str>) -> ModpackListItem {
        ModpackListItem {
            updated_at: at.map(str::to_string),
            ..item(name, source, 0)
        }
    }

    fn names(page: &ModpackSearchPage) -> Vec<&str> {
        page.items.iter().map(|item| item.name.as_str()).collect()
    }

    fn two_sources(
        modrinth: Vec<ModpackListItem>,
        curseforge: Vec<ModpackListItem>,
    ) -> Vec<SourcePage> {
        vec![
            SourcePage {
                items: modrinth,
                total: 25,
            },
            SourcePage {
                items: curseforge,
                total: 12,
            },
        ]
    }

    #[test]
    fn merges_both_sources_sorted_by_downloads() {
        let page = merge_search_pages(
            2,
            two_sources(
                vec![item("a", ModpackSource::Modrinth, 5)],
                vec![item("b", ModpackSource::Curseforge, 9)],
            ),
            ModpackSort::Downloads,
        );
        assert_eq!(names(&page), vec!["b", "a"]);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_count, 3);
    }

    #[test]
    fn sorts_by_last_update_with_undated_items_last() {
        let page = merge_search_pages(
            1,
            two_sources(
                vec![
                    updated(
                        "old",
                        ModpackSource::Modrinth,
                        Some("2026-01-02T00:00:00.5Z"),
                    ),
                    updated("undated", ModpackSource::Modrinth, None),
                ],
                vec![updated(
                    "new",
                    ModpackSource::Curseforge,
                    Some("2026-11-08T00:51:06.56Z"),
                )],
            ),
            ModpackSort::Updated,
        );
        assert_eq!(names(&page), vec!["new", "old", "undated"]);
    }

    #[test]
    fn relevance_takes_turns_and_keeps_each_source_order() {
        let page = merge_search_pages(
            1,
            two_sources(
                vec![
                    item("m1", ModpackSource::Modrinth, 1),
                    item("m2", ModpackSource::Modrinth, 2),
                    item("m3", ModpackSource::Modrinth, 3),
                ],
                vec![item("c1", ModpackSource::Curseforge, 9)],
            ),
            ModpackSort::Relevance,
        );
        assert_eq!(names(&page), vec!["m1", "c1", "m2", "m3"]);
    }

    #[test]
    fn no_hits_means_no_pages() {
        let page = merge_search_pages(
            1,
            vec![SourcePage {
                items: vec![],
                total: 0,
            }],
            ModpackSort::Downloads,
        );
        assert!(page.items.is_empty());
        assert_eq!(page.page_count, 0);
    }

    #[test]
    fn the_source_filter_picks_which_apis_are_searched() {
        let both = ModpackSearchFilters::default();
        assert!(searches_modrinth(&both) && searches_curseforge(&both));

        let modrinth_only = ModpackSearchFilters {
            source: Some(ModpackSource::Modrinth),
            ..ModpackSearchFilters::default()
        };
        assert!(searches_modrinth(&modrinth_only) && !searches_curseforge(&modrinth_only));

        let curseforge_only = ModpackSearchFilters {
            source: Some(ModpackSource::Curseforge),
            ..ModpackSearchFilters::default()
        };
        assert!(!searches_modrinth(&curseforge_only) && searches_curseforge(&curseforge_only));
    }

    #[test]
    fn a_category_curseforge_lacks_leaves_it_out() {
        let optimization = ModpackSearchFilters {
            category: Some(ModpackCategory::Optimization),
            ..ModpackSearchFilters::default()
        };
        assert!(searches_modrinth(&optimization) && !searches_curseforge(&optimization));

        let magic = ModpackSearchFilters {
            category: Some(ModpackCategory::Magic),
            ..ModpackSearchFilters::default()
        };
        assert!(searches_curseforge(&magic));
    }

    fn page_of(name: &str) -> SourcePage {
        SourcePage {
            items: vec![item(name, ModpackSource::Curseforge, 1)],
            total: 1,
        }
    }

    #[test]
    fn a_failing_curseforge_leaves_modrinth_results_when_both_are_searched() {
        let failed = curseforge_outcome(Err(CoreError::CurseforgeKeyRejected), false);
        assert!(matches!(failed, Ok(None)));
        let missing = curseforge_outcome(Err(CoreError::CurseforgeKeyMissing), false);
        assert!(matches!(missing, Ok(None)));
        let found = curseforge_outcome(Ok(page_of("c")), false).expect("ok");
        assert_eq!(found.map(|page| page.items.len()), Some(1));
    }

    #[test]
    fn a_curseforge_only_search_reports_why_it_failed() {
        assert!(matches!(
            curseforge_outcome(Err(CoreError::CurseforgeKeyRejected), true),
            Err(CoreError::CurseforgeKeyRejected)
        ));
        assert!(matches!(
            curseforge_outcome(Err(CoreError::CurseforgeKeyMissing), true),
            Err(CoreError::CurseforgeKeyMissing)
        ));
    }

    #[tokio::test]
    async fn page_zero_is_rejected_before_any_request() {
        let client = Client::new();
        let filters = ModpackSearchFilters::default();
        assert!(search_modpacks(&client, "", 0, &filters).await.is_err());
    }
}
