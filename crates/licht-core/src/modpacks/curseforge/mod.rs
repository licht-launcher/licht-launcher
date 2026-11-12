//! CurseForge modpack search, pack download and install.

mod api;
mod cdn;
mod install;
pub(crate) mod storage;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use serde_json::Value;
use url::Url;

use std::collections::HashMap;

use tokio::sync::OnceCell;

use crate::modpacks::types::{
    ModpackCategory, ModpackDetails, ModpackSearchFilters, ModpackVersionItem, SourcePage,
};
use crate::{CoreError, Result};

pub(crate) use install::import_curseforge_into;
pub use install::install_curseforge_files;

const SEARCH_URL: &str = "https://api.curseforge.com/v1/mods/search";
const MOD_URL: &str = "https://api.curseforge.com/v1/mods";
const FILES_URL: &str = "https://api.curseforge.com/v1/mods/files";
const CATEGORIES_URL: &str = "https://api.curseforge.com/v1/categories";
const GAME_ID: &str = "432";
const CLASS_ID: &str = "4471";

pub const CURSEFORGE_API_KEY_ENV: &str = "LICHT_CURSEFORGE_API_KEY";

/// Legacy per-instance list of resolved CurseForge files, imported into SQLite on first use.
pub const CURSEFORGE_INDEX_FILE: &str = "licht-curseforge.json";

pub fn curseforge_api_key() -> Result<String> {
    match std::env::var(CURSEFORGE_API_KEY_ENV) {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(CoreError::CurseforgeKeyMissing),
    }
}

/// Modpack category ids by slug, loaded once per run (they rarely change).
static MODPACK_CATEGORIES: OnceCell<HashMap<String, u64>> = OnceCell::const_new();

/// Whether a CurseForge search can honor this category filter.
pub(super) fn serves_category(category: ModpackCategory) -> bool {
    api::category_slug(category).is_some()
}

pub(super) async fn search_curseforge(
    client: &Client,
    query: &str,
    index: u64,
    page_size: u32,
    filters: &ModpackSearchFilters,
) -> Result<SourcePage> {
    let key = curseforge_api_key()?;
    if !api::fits_search_window(index, page_size) {
        return Ok(api::past_search_window());
    }
    let mut category_ids = Vec::with_capacity(filters.categories.len());
    for &category in &filters.categories {
        match category_id(client, &key, category).await? {
            Some(id) => category_ids.push(id),
            None => {
                tracing::warn!(
                    ?category,
                    "CurseForge has no matching modpack category; leaving it out of this search"
                );
                return Ok(SourcePage {
                    items: Vec::new(),
                    total: 0,
                });
            }
        }
    }
    let url = search_url(query, index, page_size, filters, &category_ids)?;
    let body = api::get_api(client, &key, url.as_str(), "curseforge_search").await?;
    api::parse_search(&body)
}

async fn category_id(client: &Client, key: &str, category: ModpackCategory) -> Result<Option<u64>> {
    let Some(slug) = api::category_slug(category) else {
        return Ok(None);
    };
    let ids = MODPACK_CATEGORIES
        .get_or_try_init(|| async {
            let url = format!("{CATEGORIES_URL}?gameId={GAME_ID}&classId={CLASS_ID}");
            let body = api::get_api(client, key, &url, "curseforge_categories").await?;
            api::parse_categories(&body)
        })
        .await?;
    Ok(ids.get(slug).copied())
}

fn search_url(
    query: &str,
    index: u64,
    page_size: u32,
    filters: &ModpackSearchFilters,
    category_ids: &[u64],
) -> Result<Url> {
    let mut url = Url::parse(SEARCH_URL).map_err(|_| CoreError::ModpackMeta {
        detail: "curseforge_search_url".into(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("gameId", GAME_ID);
        pairs.append_pair("classId", CLASS_ID);
        pairs.append_pair("sortField", api::sort_field(filters.sort));
        pairs.append_pair("sortOrder", "desc");
        if let Some(version) = filters.game_version() {
            pairs.append_pair("gameVersion", version);
        }
        if let Some(loader) = filters.loader {
            pairs.append_pair("modLoaderType", api::mod_loader_type(loader));
        }
        if !category_ids.is_empty() {
            // CurseForge returns only packs that have every listed category.
            let ids: Vec<String> = category_ids.iter().map(u64::to_string).collect();
            pairs.append_pair("categoryIds", &format!("[{}]", ids.join(",")));
        }
        pairs.append_pair("index", &index.to_string());
        pairs.append_pair("pageSize", &page_size.to_string());
        if !query.trim().is_empty() {
            pairs.append_pair("searchFilter", query.trim());
        }
    }
    Ok(url)
}

pub(super) async fn get_curseforge_mod(client: &Client, mod_id: &str) -> Result<ModpackDetails> {
    let key = curseforge_api_key()?;
    let mod_id = parse_mod_id(mod_id)?;
    let mod_url = format!("{MOD_URL}/{mod_id}");
    let description_url = format!("{MOD_URL}/{mod_id}/description");
    let (project, description) = tokio::join!(
        api::get_api(client, &key, &mod_url, "curseforge_mod"),
        api::get_api(client, &key, &description_url, "curseforge_description"),
    );
    api::parse_mod(&project?, &description?)
}

/// Files fetched per request; CurseForge's maximum page size.
const FILES_PAGE_SIZE: u64 = 50;
/// At most this many file pages (1,000 files) per pack, to bound requests.
const MAX_FILE_PAGES: u64 = 20;

/// File pages requested at once after the first one.
const FILE_PAGE_CONCURRENCY: usize = 4;

/// Every file of a pack. The first page carries the total, so the other pages
/// are then fetched a few at a time instead of one after another.
pub(super) async fn list_curseforge_versions(
    client: &Client,
    mod_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    let key = curseforge_api_key()?;
    let key = key.as_str();
    let mod_id = parse_mod_id(mod_id)?;
    let fetch_page = move |index: u64| async move {
        let url = format!("{MOD_URL}/{mod_id}/files?index={index}&pageSize={FILES_PAGE_SIZE}");
        let body = api::get_api(client, key, &url, "curseforge_files").await?;
        api::parse_versions(&body)
    };
    let first = fetch_page(0).await?;
    if first.total > MAX_FILE_PAGES * FILES_PAGE_SIZE {
        tracing::warn!(
            mod_id,
            total = first.total,
            "CurseForge pack has more files than Licht lists"
        );
    }
    let others: Vec<api::FilesPage> = stream::iter(remaining_file_indexes(first.total))
        .map(fetch_page)
        .buffered(FILE_PAGE_CONCURRENCY)
        .try_collect()
        .await?;
    let mut versions = first.items;
    for page in others {
        versions.extend(page.items);
    }
    Ok(versions)
}

/// Start indexes of the file pages after the first, up to `MAX_FILE_PAGES`.
fn remaining_file_indexes(total: u64) -> Vec<u64> {
    (1..MAX_FILE_PAGES)
        .map(|page| page * FILES_PAGE_SIZE)
        .take_while(|index| *index < total)
        .collect()
}

fn parse_mod_id(mod_id: &str) -> Result<u64> {
    mod_id.parse::<u64>().map_err(|_| CoreError::ModpackMeta {
        detail: "invalid modpack identifier".into(),
    })
}

pub(super) async fn download_curseforge_pack_to(
    client: &Client,
    mod_id: &str,
    file_id: &str,
    destination: &std::path::Path,
    context: &crate::InstallationContext,
) -> Result<()> {
    let key = curseforge_api_key()?;
    let url = format!("{MOD_URL}/{mod_id}/files/{file_id}/download-url");
    let download_url = context
        .metadata(async {
            let response = client
                .get(&url)
                .header("x-api-key", &key)
                .header("Accept", "application/json")
                .send()
                .await?;
            let status = response.status().as_u16();
            let body = response.text().await?;
            if !(200..300).contains(&status) {
                return Err(download_url_status_error(status, &body));
            }
            parse_pack_download_url(&body)
        })
        .await?;
    let parse = |value: &str| {
        value.parse::<u64>().map_err(|_| CoreError::ModpackMeta {
            detail: "invalid modpack identifier".into(),
        })
    };
    let (project, file_id) = (parse(mod_id)?, parse(file_id)?);
    let files = context
        .metadata(api::fetch_remote_files(client, &key, &[file_id]))
        .await?;
    let file = files
        .iter()
        .find(|file| file.file_id == file_id && file.mod_id == project)
        .ok_or(CoreError::CurseforgeFileMissing { file_id })?;
    let checksum = match (&file.sha1, &file.md5) {
        (Some(value), _) => crate::DownloadChecksum::Sha1(value.clone()),
        (_, Some(value)) => crate::DownloadChecksum::Md5(value.clone()),
        _ => {
            return Err(CoreError::ModpackMeta {
                detail: "modpack archive checksum is missing".into(),
            });
        }
    };
    let task = crate::VerifiedDownloadTask {
        url: download_url,
        destination: destination.to_path_buf(),
        checksum,
        size: file.size,
    };
    context
        .verified_queue(&cdn::cdn_client(&key)?, &[task], context.options().await?)
        .await?;
    context.stage(crate::DownloadPhase::Verifying).await?;
    if let Some(expected) = &file.md5 {
        super::file_hashes::ensure_md5(destination, expected)?;
    }
    Ok(())
}

fn download_url_status_error(status: u16, body: &str) -> CoreError {
    if status == 403 && body.to_ascii_lowercase().contains("distribution") {
        return CoreError::CurseforgePackDistributionDenied;
    }
    CoreError::ModpackMeta {
        detail: format!("curseforge_download_url_http_{status}"),
    }
}

fn parse_pack_download_url(body: &str) -> Result<String> {
    let parsed: Value = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    match parsed.get("data") {
        Some(Value::Null) => Err(CoreError::CurseforgePackDistributionDenied),
        Some(Value::String(url)) => cdn::canonical_download_url(url),
        _ => Err(CoreError::ModpackMeta {
            detail: "curseforge_download_url_missing".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_files_page_says_which_others_to_fetch() {
        assert_eq!(remaining_file_indexes(148), vec![50, 100]);
        assert_eq!(remaining_file_indexes(50), Vec::<u64>::new());
        assert_eq!(remaining_file_indexes(51), vec![50]);
        // No total in the reply: the first page is all there is.
        assert_eq!(remaining_file_indexes(0), Vec::<u64>::new());
    }

    #[test]
    fn file_pages_stop_at_the_cap() {
        let indexes = remaining_file_indexes(5_000);
        assert_eq!(
            u64::try_from(indexes.len()).expect("len"),
            MAX_FILE_PAGES - 1
        );
        assert_eq!(indexes.last(), Some(&950));
    }

    #[test]
    fn search_url_pages_with_index_and_page_size() {
        let url = search_url("atm", 20, 10, &ModpackSearchFilters::default(), &[]).expect("url");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("index".into(), "20".into())));
        assert!(pairs.contains(&("pageSize".into(), "10".into())));
        assert!(pairs.contains(&("searchFilter".into(), "atm".into())));
    }

    #[test]
    fn filters_become_curseforge_search_parameters() {
        let filters = ModpackSearchFilters {
            game_version: Some("1.20.1".into()),
            loader: Some(crate::modpacks::types::ModpackLoader::Fabric),
            categories: vec![ModpackCategory::Technology, ModpackCategory::Magic],
            source: None,
            sort: crate::modpacks::types::ModpackSort::Updated,
        };
        let url = search_url("", 0, 10, &filters, &[4472, 4473]).expect("url");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("sortField".into(), "3".into())));
        assert!(pairs.contains(&("gameVersion".into(), "1.20.1".into())));
        assert!(pairs.contains(&("modLoaderType".into(), "4".into())));
        assert!(pairs.contains(&("categoryIds".into(), "[4472,4473]".into())));
    }

    #[test]
    fn sorting_by_downloads_uses_total_downloads_without_extra_filters() {
        let url = search_url("", 0, 10, &ModpackSearchFilters::default(), &[]).expect("url");
        let keys: Vec<String> = url.query_pairs().map(|(key, _)| key.into_owned()).collect();
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "sortField" && value == "6")
        );
        for absent in [
            "gameVersion",
            "modLoaderType",
            "categoryIds",
            "searchFilter",
        ] {
            assert!(!keys.iter().any(|key| key == absent), "{absent}");
        }
    }

    #[test]
    fn pack_download_url_must_be_a_cdn_host() {
        let url = parse_pack_download_url(
            r#"{"data":"https://edge.forgecdn.net/files/1/2/pack.zip?api-key=secret"}"#,
        )
        .expect("url");
        assert!(!url.contains("secret"));
        assert!(url.starts_with("https://edge.forgecdn.net/"));
        let denied = parse_pack_download_url(r#"{"data":null}"#).unwrap_err();
        assert!(matches!(
            denied,
            CoreError::CurseforgePackDistributionDenied
        ));
        let github =
            parse_pack_download_url(r#"{"data":"https://github.com/a/pack.zip"}"#).unwrap_err();
        assert!(matches!(github, CoreError::CurseforgeDownloadHost));
    }

    #[test]
    fn a_403_that_mentions_distribution_is_the_pack_error() {
        let error = download_url_status_error(403, "Project distribution is disabled");
        assert!(matches!(error, CoreError::CurseforgePackDistributionDenied));
        let other = download_url_status_error(403, "bad key");
        match other {
            CoreError::ModpackMeta { detail } => {
                assert_eq!(detail, "curseforge_download_url_http_403");
            }
            other => panic!("expected metadata failure, got {other}"),
        }
    }
}
