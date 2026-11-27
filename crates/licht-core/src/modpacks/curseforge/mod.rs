//! CurseForge modpack search, pack download and install.

mod api;
mod cdn;
mod install;
pub(crate) mod storage;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use serde_json::Value;
use url::Url;

use std::collections::{HashMap, HashSet};

use tokio::sync::OnceCell;

use crate::modpacks::types::{
    ModpackCategory, ModpackDetails, ModpackListItem, ModpackLoader, ModpackSearchFilters,
    ModpackSort, ModpackVersionItem, SourcePage,
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
    // The search endpoint takes one modLoaderType, so each loader is its own request.
    let selections: Vec<Option<ModpackLoader>> = if filters.loaders.is_empty() {
        vec![None]
    } else {
        filters.loaders.iter().copied().map(Some).collect()
    };
    let pages = futures_util::future::try_join_all(selections.into_iter().map(|loader| {
        let category_ids = category_ids.clone();
        let key = key.clone();
        async move {
            let url = search_url(query, index, page_size, filters, &category_ids, loader)?;
            let body = api::get_api(client, &key, url.as_str(), "curseforge_search").await?;
            api::parse_search(&body)
        }
    }))
    .await?;
    Ok(merge_loader_pages(pages, filters.sort))
}

fn merge_loader_pages(mut pages: Vec<SourcePage>, sort: ModpackSort) -> SourcePage {
    if pages.len() <= 1 {
        return pages.pop().unwrap_or(SourcePage {
            items: Vec::new(),
            total: 0,
        });
    }
    // A pack that matches two loaders is counted in both totals.
    let total = pages.iter().map(|page| page.total).sum();
    let lists: Vec<Vec<ModpackListItem>> = pages.into_iter().map(|page| page.items).collect();
    // Downloads and updated are ordered again when the sources are merged.
    // Relevance is not, so the loader pages have to take turns here.
    let ordered = if sort == ModpackSort::Relevance {
        super::discovery::interleave(lists)
    } else {
        lists.into_iter().flatten().collect()
    };
    let mut seen = HashSet::new();
    let items = ordered
        .into_iter()
        .filter(|item| seen.insert(item.id.clone()))
        .collect();
    SourcePage { items, total }
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
    loader: Option<ModpackLoader>,
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
        if let Some(loader) = loader {
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
        api::get_api(client, key, &url, "curseforge_files").await
    };
    let (first, title) = tokio::join!(fetch_page(0), fetch_pack_title(client, key, mod_id));
    let title = title_or_none(title, mod_id);
    let title = title.as_str();
    let first = api::parse_versions(&first?, title)?;
    if first.total > MAX_FILE_PAGES * FILES_PAGE_SIZE {
        tracing::warn!(
            mod_id,
            total = first.total,
            "CurseForge pack has more files than Licht lists"
        );
    }
    let others: Vec<String> = stream::iter(remaining_file_indexes(first.total))
        .map(fetch_page)
        .buffered(FILE_PAGE_CONCURRENCY)
        .try_collect()
        .await?;
    let mut versions = first.items;
    for body in others {
        versions.extend(api::parse_versions(&body, title)?.items);
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
) -> Result<super::PackVersion> {
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
    let file_ids = [file_id];
    let (files, title) = context
        .metadata(async {
            let (files, title) = tokio::join!(
                api::fetch_remote_files(client, &key, &file_ids),
                fetch_pack_title(client, &key, project),
            );
            Ok::<_, CoreError>((files?, title_or_none(title, project)))
        })
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
    Ok(pack_version(file, &title))
}

/// A pack file's display name stands for its version, like the file list shows it.
fn pack_version(file: &api::RemoteFile, pack_title: &str) -> super::PackVersion {
    let name = if file.display_name.trim().is_empty() {
        file.file_name.clone()
    } else {
        version_label(&file.display_name, pack_title)
    };
    super::PackVersion {
        name,
        published_at: file.file_date.clone(),
    }
}

/// Separators that set a subtitle apart in a pack title ("Homestead - A Cozy Pack").
const SUBTITLE_SEPARATORS: [&str; 4] = [" - ", " – ", ":", " | "];

/// CurseForge file names usually repeat the pack title ("Homestead 2.0.1");
/// the version is what follows it, or what follows the title without its
/// subtitle. A name without the title stays whole.
fn version_label(name: &str, pack_title: &str) -> String {
    let name = name.trim();
    let title = pack_title.trim();
    let short_title = SUBTITLE_SEPARATORS
        .iter()
        .filter_map(|separator| title.find(separator))
        .min()
        .map(|end| title[..end].trim());
    strip_title(name, title)
        .or_else(|| short_title.and_then(|short| strip_title(name, short)))
        .unwrap_or(name)
        .to_string()
}

/// What follows `title` at the start of `name`, ignoring case, when the title
/// ends at a word boundary and something is left.
fn strip_title<'a>(name: &'a str, title: &str) -> Option<&'a str> {
    if title.is_empty() {
        return None;
    }
    let title_chars = title.chars().count();
    let end = name
        .char_indices()
        .nth(title_chars)
        .map_or(name.len(), |(index, _)| index);
    let head = &name[..end];
    if head.chars().count() != title_chars || head.to_lowercase() != title.to_lowercase() {
        return None;
    }
    let rest = &name[end..];
    rest.starts_with(|ch: char| !ch.is_alphanumeric())
        .then(|| rest.trim_start_matches(|ch: char| !ch.is_alphanumeric()))
        .filter(|rest| !rest.is_empty())
}

/// The title only tidies version names, so a failed lookup leaves them whole
/// instead of failing the listing or the install.
fn title_or_none(title: Result<String>, mod_id: u64) -> String {
    title.unwrap_or_else(|error| {
        tracing::warn!(
            mod_id,
            %error,
            "CurseForge pack title lookup failed; showing full version names"
        );
        String::new()
    })
}

async fn fetch_pack_title(client: &Client, key: &str, mod_id: u64) -> Result<String> {
    let body = api::get_api(
        client,
        key,
        &format!("{MOD_URL}/{mod_id}"),
        "curseforge_mod",
    )
    .await?;
    api::parse_mod_title(&body)
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

    fn remote_file(display_name: &str) -> api::RemoteFile {
        api::RemoteFile {
            file_id: 2,
            mod_id: 1,
            file_name: "pack-1.0.zip".into(),
            display_name: display_name.into(),
            file_date: Some("2026-11-04T07:16:05.953Z".into()),
            size: 10,
            sha1: None,
            md5: None,
            download_url: None,
        }
    }

    #[test]
    fn a_pack_file_stands_for_its_version_by_display_name() {
        assert_eq!(
            pack_version(&remote_file("Pack 1.0"), "Pack"),
            super::super::PackVersion {
                name: "1.0".into(),
                published_at: Some("2026-11-04T07:16:05.953Z".into()),
            }
        );
        assert_eq!(pack_version(&remote_file(" "), "Pack").name, "pack-1.0.zip");
    }

    #[test]
    fn a_version_drops_the_pack_title_it_starts_with() {
        assert_eq!(version_label("Homestead 2.0.1", "Homestead"), "2.0.1");
        assert_eq!(version_label("homestead-1.2.9.4", "Homestead"), "1.2.9.4");
        assert_eq!(
            version_label("All the Mods 9 - 0.3.2", "All the Mods 9"),
            "0.3.2"
        );
        // The title must end at a word boundary.
        assert_eq!(
            version_label("Homesteadders 1.0", "Homestead"),
            "Homesteadders 1.0"
        );
        // Nothing left after the title: keep the whole name.
        assert_eq!(version_label("Homestead", "Homestead"), "Homestead");
        assert_eq!(version_label("2.0.1", "Homestead"), "2.0.1");
        assert_eq!(version_label("Ñandú 1.0", "Ñandú"), "1.0");
        assert_eq!(version_label("ñandú 1.0", "Ñandú"), "1.0");
        assert_eq!(version_label("ñ 1.0", "Ñandú"), "ñ 1.0");
    }

    #[test]
    fn a_version_drops_the_short_title_before_a_subtitle() {
        assert_eq!(
            version_label("Homestead 2.0.1", "Homestead - A Cozy Pack"),
            "2.0.1"
        );
        assert_eq!(version_label("Pack 1.0", "Pack: Reloaded"), "1.0");
        // The full title wins when the name carries it.
        assert_eq!(version_label("Pack: Reloaded 3.0", "Pack: Reloaded"), "3.0");
    }

    #[test]
    fn a_failed_title_lookup_leaves_names_whole() {
        let title = title_or_none(
            Err(CoreError::ModpackMeta {
                detail: "curseforge_mod_http_429".into(),
            }),
            1,
        );
        assert_eq!(title, "");
        assert_eq!(version_label("Homestead 2.0.1", &title), "Homestead 2.0.1");
        assert_eq!(title_or_none(Ok("Homestead".into()), 1), "Homestead");
    }

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
        let url =
            search_url("atm", 20, 10, &ModpackSearchFilters::default(), &[], None).expect("url");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("index".into(), "20".into())));
        assert!(pairs.contains(&("pageSize".into(), "10".into())));
        assert!(pairs.contains(&("searchFilter".into(), "atm".into())));
    }

    #[test]
    fn filters_become_curseforge_search_parameters() {
        let filters = ModpackSearchFilters {
            game_version: Some("1.20.1".into()),
            loaders: vec![crate::modpacks::types::ModpackLoader::Fabric],
            categories: vec![ModpackCategory::Technology, ModpackCategory::Magic],
            source: None,
            sort: crate::modpacks::types::ModpackSort::Updated,
        };
        let url = search_url(
            "",
            0,
            10,
            &filters,
            &[4472, 4473],
            Some(crate::modpacks::types::ModpackLoader::Fabric),
        )
        .expect("url");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("sortField".into(), "3".into())));
        assert!(pairs.contains(&("gameVersion".into(), "1.20.1".into())));
        assert!(pairs.contains(&("modLoaderType".into(), "4".into())));
        assert!(pairs.contains(&("categoryIds".into(), "[4472,4473]".into())));
    }

    #[test]
    fn sorting_by_downloads_uses_total_downloads_without_extra_filters() {
        let url = search_url("", 0, 10, &ModpackSearchFilters::default(), &[], None).expect("url");
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
    fn merged_loader_pages_keep_one_copy_of_a_pack_and_sum_the_totals() {
        let merged = merge_loader_pages(
            vec![
                SourcePage {
                    items: vec![list_item("a", 10), list_item("b", 30)],
                    total: 2,
                },
                SourcePage {
                    items: vec![list_item("b", 30), list_item("c", 5)],
                    total: 4,
                },
            ],
            ModpackSort::Downloads,
        );
        assert_eq!(merged.total, 6);
        let ids: Vec<&str> = merged.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
    }

    #[test]
    fn merged_loader_pages_take_turns_when_sorted_by_relevance() {
        let merged = merge_loader_pages(
            vec![
                SourcePage {
                    items: vec![list_item("a", 1), list_item("c", 1)],
                    total: 2,
                },
                SourcePage {
                    items: vec![list_item("b", 1)],
                    total: 1,
                },
            ],
            ModpackSort::Relevance,
        );
        let ids: Vec<&str> = merged.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
    }

    fn list_item(id: &str, downloads: u64) -> crate::modpacks::types::ModpackListItem {
        crate::modpacks::types::ModpackListItem {
            id: id.into(),
            source: crate::modpacks::types::ModpackSource::Curseforge,
            name: id.into(),
            description: String::new(),
            downloads,
            icon_url: None,
            author: String::new(),
            updated_at: None,
            loaders: Vec::new(),
            categories: Vec::new(),
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
