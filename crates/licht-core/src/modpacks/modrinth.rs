use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::modpacks::description::markdown_to_safe_html;
use crate::modpacks::types::{
    ModpackCategory, ModpackDetails, ModpackListItem, ModpackLoader, ModpackSearchFilters,
    ModpackSort, ModpackSource, ModpackVersionItem, SourcePage, push_unique,
};
use crate::{CoreError, Result};

const SEARCH_URL: &str = "https://api.modrinth.com/v2/search";
const PROJECT_URL: &str = "https://api.modrinth.com/v2/project";
const USER_AGENT: &str = "licht-launcher/0.1.1 (https://github.com/licht-launcher/licht-launcher)";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
    #[serde(default)]
    total_hits: u64,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    project_id: String,
    title: String,
    description: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    author: String,
    #[serde(default)]
    date_modified: Option<String>,
    /// Modrinth lists loaders among the categories.
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectResponse {
    id: String,
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    body: String,
    project_type: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    updated: Option<String>,
    #[serde(default)]
    gallery: Vec<GalleryImage>,
}

#[derive(Debug, Deserialize)]
struct GalleryImage {
    url: String,
}

pub(super) async fn search_modrinth(
    client: &Client,
    query: &str,
    offset: u64,
    limit: u32,
    filters: &ModpackSearchFilters,
) -> Result<SourcePage> {
    let url = search_url(query, offset, limit, filters)?;
    let body = fetch_text(client, url, "modrinth_search").await?;
    parse_search(&body)
}

fn search_url(query: &str, offset: u64, limit: u32, filters: &ModpackSearchFilters) -> Result<Url> {
    let mut url = Url::parse(SEARCH_URL).map_err(|_| CoreError::ModpackMeta {
        detail: "modrinth_search_url".into(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("limit", &limit.to_string());
        pairs.append_pair("offset", &offset.to_string());
        pairs.append_pair("index", sort_index(filters.sort));
        pairs.append_pair("facets", &search_facets(filters).to_string());
        if !query.trim().is_empty() {
            pairs.append_pair("query", query.trim());
        }
    }
    Ok(url)
}

/// Facets are ANDed between the inner lists; loaders are categories on Modrinth.
fn search_facets(filters: &ModpackSearchFilters) -> Value {
    let mut facets = vec![vec!["project_type:modpack".to_string()]];
    if let Some(version) = filters.game_version() {
        facets.push(vec![format!("versions:{version}")]);
    }
    if let Some(loader) = filters.loader {
        facets.push(vec![format!("categories:{}", loader_slug(loader))]);
    }
    if let Some(category) = filters.category {
        facets.push(vec![format!("categories:{}", category_slug(category))]);
    }
    serde_json::json!(facets)
}

fn sort_index(sort: ModpackSort) -> &'static str {
    match sort {
        ModpackSort::Downloads => "downloads",
        ModpackSort::Updated => "updated",
        ModpackSort::Relevance => "relevance",
    }
}

fn loader_slug(loader: ModpackLoader) -> &'static str {
    match loader {
        ModpackLoader::Fabric => "fabric",
        ModpackLoader::Forge => "forge",
        ModpackLoader::Neoforge => "neoforge",
        ModpackLoader::Quilt => "quilt",
    }
}

/// Modrinth's modpack categories (`GET /v2/tag/category`); the shared list mirrors them.
fn category_slug(category: ModpackCategory) -> &'static str {
    match category {
        ModpackCategory::Adventure => "adventure",
        ModpackCategory::Challenging => "challenging",
        ModpackCategory::Combat => "combat",
        ModpackCategory::KitchenSink => "kitchen-sink",
        ModpackCategory::Lightweight => "lightweight",
        ModpackCategory::Magic => "magic",
        ModpackCategory::Multiplayer => "multiplayer",
        ModpackCategory::Optimization => "optimization",
        ModpackCategory::Quests => "quests",
        ModpackCategory::Technology => "technology",
    }
}

pub(super) fn parse_search(body: &str) -> Result<SourcePage> {
    let parsed: SearchResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    let items = parsed
        .hits
        .into_iter()
        .map(|hit| {
            let mut loaders = Vec::new();
            let mut categories = Vec::new();
            for slug in &hit.categories {
                if let Some(loader) = ModpackLoader::ALL
                    .into_iter()
                    .find(|loader| loader_slug(*loader) == slug)
                {
                    push_unique(&mut loaders, loader);
                } else if let Some(category) = ModpackCategory::ALL
                    .into_iter()
                    .find(|category| category_slug(*category) == slug)
                {
                    push_unique(&mut categories, category);
                }
            }
            ModpackListItem {
                id: hit.project_id,
                source: ModpackSource::Modrinth,
                name: hit.title,
                description: hit.description,
                downloads: hit.downloads,
                icon_url: hit.icon_url,
                author: hit.author,
                updated_at: hit.date_modified,
                loaders,
                categories,
            }
        })
        .collect();
    Ok(SourcePage {
        items,
        total: parsed.total_hits,
    })
}

/// The project endpoint has no author name for organization-owned packs, so the
/// author comes from the same search index the listing shows.
pub(super) async fn get_modrinth_project(
    client: &Client,
    project_id: &str,
) -> Result<ModpackDetails> {
    let project_endpoint = project_endpoint(project_id)?;
    let author_url = author_search_url(project_id)?;
    let (project, author) = tokio::join!(
        fetch_text(client, project_endpoint, "modrinth_project"),
        fetch_text(client, author_url, "modrinth_author"),
    );
    let author = parse_search(&author?)?
        .items
        .into_iter()
        .next()
        .map(|item| item.author)
        .unwrap_or_default();
    parse_project(&project?, author)
}

fn project_endpoint(project_id: &str) -> Result<Url> {
    let invalid = || CoreError::ModpackMeta {
        detail: "modrinth_project_id".into(),
    };
    if project_id.trim().is_empty() {
        return Err(invalid());
    }
    let mut url = Url::parse(PROJECT_URL).map_err(|_| invalid())?;
    // `push` percent-encodes, so an id can never add path segments.
    url.path_segments_mut()
        .map_err(|_| invalid())?
        .push(project_id);
    Ok(url)
}

fn author_search_url(project_id: &str) -> Result<Url> {
    let mut url = Url::parse(SEARCH_URL).map_err(|_| CoreError::ModpackMeta {
        detail: "modrinth_search_url".into(),
    })?;
    let facets = serde_json::json!([[format!("project_id:{project_id}")]]);
    url.query_pairs_mut()
        .append_pair("limit", "1")
        .append_pair("facets", &facets.to_string());
    Ok(url)
}

pub(super) fn parse_project(body: &str, author: String) -> Result<ModpackDetails> {
    let project: ProjectResponse = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    if project.project_type != "modpack" {
        return Err(CoreError::ModpackMeta {
            detail: "modrinth_project_not_modpack".into(),
        });
    }
    Ok(ModpackDetails {
        id: project.id,
        source: ModpackSource::Modrinth,
        name: project.title,
        summary: project.description,
        author,
        downloads: project.downloads,
        icon_url: project.icon_url,
        categories: project.categories,
        updated_at: project.updated,
        gallery: project
            .gallery
            .into_iter()
            .map(|image| image.url)
            .filter(|url| url.starts_with("https://"))
            .collect(),
        project_url: Some(format!("https://modrinth.com/modpack/{}", project.slug)),
        description_html: markdown_to_safe_html(&project.body),
    })
}

async fn fetch_text(client: &Client, url: Url, label: &str) -> Result<String> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("{label}_http_{status}"),
        });
    }
    Ok(body)
}

pub async fn list_modrinth_versions(
    client: &Client,
    project_id: &str,
) -> Result<Vec<ModpackVersionItem>> {
    let url = format!("{PROJECT_URL}/{project_id}/version");
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("modrinth_versions_http_{status}"),
        });
    }
    parse_versions(&body)
}

pub fn parse_versions(body: &str) -> Result<Vec<ModpackVersionItem>> {
    let parsed: Vec<Value> = serde_json::from_str(body).map_err(CoreError::LoaderJson)?;
    Ok(parsed
        .into_iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let game_versions = entry
                .get("game_versions")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let loaders = entry
                .get("loaders")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(ModpackVersionItem {
                id,
                name,
                game_versions,
                loaders,
            })
        })
        .collect())
}

pub(super) async fn download_modrinth_mrpack_to(
    client: &Client,
    project_id: &str,
    version_id: &str,
    destination: &std::path::Path,
    context: &crate::InstallationContext,
) -> Result<()> {
    let versions = context
        .metadata(list_modrinth_versions(client, project_id))
        .await?;
    if !versions.iter().any(|version| version.id == version_id) {
        return Err(CoreError::ModpackMeta {
            detail: "version_not_in_project".into(),
        });
    }
    let url = format!("{PROJECT_URL}/{project_id}/version/{version_id}");
    let parsed = context
        .metadata(async {
            let response = client
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await?;
            let status = response.status().as_u16();
            let body = response.text().await?;
            if !(200..300).contains(&status) {
                return Err(CoreError::ModpackMeta {
                    detail: format!("modrinth_version_http_{status}"),
                });
            }
            serde_json::from_str::<Value>(&body).map_err(CoreError::LoaderJson)
        })
        .await?;
    let file = parsed
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files
                .iter()
                .find(|file| file.get("primary").and_then(Value::as_bool) == Some(true))
                .or_else(|| files.first())
        })
        .ok_or(CoreError::ModpackMeta {
            detail: "mrpack_file_missing".into(),
        })?;
    let download_url = file
        .get("url")
        .and_then(Value::as_str)
        .ok_or(CoreError::ModpackMeta {
            detail: "mrpack_url_missing".into(),
        })?;
    if !super::secure_download::download_url_allowed(download_url, &["cdn.modrinth.com"]) {
        return Err(CoreError::ModpackMeta {
            detail: "modpack download host is not allowed".into(),
        });
    }
    let checksum = if let Some(value) = file.pointer("/hashes/sha1").and_then(Value::as_str) {
        crate::DownloadChecksum::Sha1(value.to_string())
    } else if let Some(value) = file.pointer("/hashes/sha512").and_then(Value::as_str) {
        crate::DownloadChecksum::Sha512(value.to_string())
    } else {
        return Err(CoreError::ModpackMeta {
            detail: "modpack archive checksum is missing".into(),
        });
    };
    let task = crate::VerifiedDownloadTask {
        url: download_url.to_string(),
        destination: destination.to_path_buf(),
        checksum,
        size: file.get("size").and_then(Value::as_u64).unwrap_or(0),
    };
    context
        .verified_queue(
            &super::mrpack::mrpack_download_client()?,
            &[task],
            context.options().await?,
        )
        .await?;
    context.stage(crate::DownloadPhase::Verifying).await?;
    if let Some(expected) = file.pointer("/hashes/sha512").and_then(Value::as_str) {
        super::file_hashes::ensure_sha512(destination, expected, &context.control).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modrinth_search_hits() {
        let body = r#"{
          "hits":[{
            "project_id":"ABC",
            "title":"Cobblemon",
            "description":"Pokemon",
            "downloads":10,
            "icon_url":null,
            "author":"team"
          }]
        }"#;
        let page = parse_search(body).expect("ok");
        assert_eq!(page.items[0].name, "Cobblemon");
        assert_eq!(page.items[0].source, ModpackSource::Modrinth);
        assert_eq!(page.total, 0);
    }

    #[test]
    fn reads_the_total_hit_count() {
        let page =
            parse_search(r#"{"hits":[],"offset":20,"limit":10,"total_hits":19217}"#).expect("ok");
        assert!(page.items.is_empty());
        assert_eq!(page.total, 19217);
    }

    #[test]
    fn search_url_pages_with_offset_and_limit() {
        let url = search_url(" cobble ", 30, 10, &ModpackSearchFilters::default()).expect("url");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("limit".into(), "10".into())));
        assert!(pairs.contains(&("offset".into(), "30".into())));
        assert!(pairs.contains(&("query".into(), "cobble".into())));
    }

    fn query_value(url: &Url, key: &str) -> Option<String> {
        url.query_pairs()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
    }

    #[test]
    fn without_filters_searches_every_modpack_by_downloads() {
        let url = search_url("", 0, 10, &ModpackSearchFilters::default()).expect("url");
        assert_eq!(query_value(&url, "index").as_deref(), Some("downloads"));
        assert_eq!(
            query_value(&url, "facets").as_deref(),
            Some(r#"[["project_type:modpack"]]"#)
        );
        assert_eq!(query_value(&url, "query"), None);
    }

    #[test]
    fn filters_become_anded_facets_and_the_sort_index() {
        let filters = ModpackSearchFilters {
            game_version: Some(" 1.20.1 ".into()),
            loader: Some(ModpackLoader::Neoforge),
            category: Some(ModpackCategory::KitchenSink),
            source: None,
            sort: ModpackSort::Updated,
        };
        let url = search_url("", 0, 10, &filters).expect("url");
        assert_eq!(query_value(&url, "index").as_deref(), Some("updated"));
        assert_eq!(
            query_value(&url, "facets").as_deref(),
            Some(
                r#"[["project_type:modpack"],["versions:1.20.1"],["categories:neoforge"],["categories:kitchen-sink"]]"#
            )
        );
    }

    #[test]
    fn a_blank_game_version_is_ignored() {
        let filters = ModpackSearchFilters {
            game_version: Some("  ".into()),
            sort: ModpackSort::Relevance,
            ..ModpackSearchFilters::default()
        };
        let url = search_url("", 0, 10, &filters).expect("url");
        assert_eq!(query_value(&url, "index").as_deref(), Some("relevance"));
        assert_eq!(
            query_value(&url, "facets").as_deref(),
            Some(r#"[["project_type:modpack"]]"#)
        );
    }

    #[test]
    fn splits_loaders_from_shared_categories() {
        let page = parse_search(
            r#"{"hits":[{"project_id":"A","title":"A","description":"","categories":["fabric","lightweight","quilt","fabric","utility","optimization"]}],"total_hits":1}"#,
        )
        .expect("ok");
        assert_eq!(
            page.items[0].loaders,
            vec![ModpackLoader::Fabric, ModpackLoader::Quilt]
        );
        assert_eq!(
            page.items[0].categories,
            vec![ModpackCategory::Lightweight, ModpackCategory::Optimization]
        );
    }

    #[test]
    fn every_loader_and_category_has_a_distinct_slug() {
        let mut slugs: Vec<&str> = ModpackLoader::ALL.into_iter().map(loader_slug).collect();
        slugs.extend(ModpackCategory::ALL.into_iter().map(category_slug));
        let count = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), count);
    }

    #[test]
    fn reads_the_last_update_of_each_hit() {
        let page = parse_search(
            r#"{"hits":[{"project_id":"A","title":"A","description":"","date_modified":"2026-11-01T10:43:19.109719Z"}],"total_hits":1}"#,
        )
        .expect("ok");
        assert_eq!(
            page.items[0].updated_at.as_deref(),
            Some("2026-11-01T10:43:19.109719Z")
        );
    }

    #[test]
    fn project_ids_cannot_escape_the_project_path() {
        let url = project_endpoint("../user/x?y").expect("url");
        assert_eq!(
            url.as_str(),
            "https://api.modrinth.com/v2/project/..%2Fuser%2Fx%3Fy"
        );
        assert!(project_endpoint(" ").is_err());
    }

    #[test]
    fn author_lookup_filters_the_search_by_project_id() {
        let url = author_search_url("1KVo5zza").expect("url");
        let facets = url
            .query_pairs()
            .find(|(key, _)| key == "facets")
            .map(|(_, value)| value.into_owned());
        assert_eq!(facets.as_deref(), Some(r#"[["project_id:1KVo5zza"]]"#));
    }

    #[test]
    fn parses_a_project_page_with_a_sanitized_body() {
        let body = r#"{
          "id":"1KVo5zza",
          "slug":"fabulously-optimized",
          "title":"Fabulously Optimized",
          "description":"Fast and pretty",
          "body":"**Hi**<script>alert(1)</script>",
          "project_type":"modpack",
          "downloads":42,
          "icon_url":"https://cdn.modrinth.com/icon.webp",
          "categories":["optimization"],
          "updated":"2026-11-01T10:43:19.109719Z",
          "gallery":[
            {"url":"https://cdn.modrinth.com/a.webp","featured":false,"ordering":1},
            {"url":"http://insecure.example/b.png","featured":false,"ordering":2}
          ]
        }"#;
        let details = parse_project(body, "robotkoer".into()).expect("ok");
        assert_eq!(details.name, "Fabulously Optimized");
        assert_eq!(details.author, "robotkoer");
        assert_eq!(details.categories, vec!["optimization"]);
        assert_eq!(details.gallery, vec!["https://cdn.modrinth.com/a.webp"]);
        assert_eq!(
            details.project_url.as_deref(),
            Some("https://modrinth.com/modpack/fabulously-optimized")
        );
        assert!(details.description_html.contains("<strong>Hi</strong>"));
        assert!(!details.description_html.contains("script"));
    }

    #[test]
    fn rejects_a_project_that_is_not_a_modpack() {
        let body = r#"{"id":"A","slug":"a","title":"A","description":"","project_type":"mod"}"#;
        assert!(parse_project(body, String::new()).is_err());
    }
}
