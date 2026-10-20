use reqwest::Client;
use serde::Deserialize;

use super::{ContentMetadata, InstanceContentKind};

pub(super) async fn fetch_curseforge_fingerprint(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    let key = std::env::var("LICHT_CURSEFORGE_API_KEY").ok()?;
    #[derive(Deserialize)]
    struct FingerprintResponse {
        data: FingerprintData,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintData {
        exact_matches: Vec<FingerprintMatch>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintMatch {
        file: FingerprintFile,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintFile {
        mod_id: u64,
        display_name: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModResponse {
        data: ModData,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModData {
        class_id: Option<u64>,
        name: String,
        logo: Option<Logo>,
    }
    #[derive(Deserialize)]
    struct Logo {
        url: Option<String>,
    }
    let response = client
        .get(format!(
            "https://api.curseforge.com/v1/fingerprints/{fingerprint}"
        ))
        .header("x-api-key", key.clone())
        .send()
        .await
        .ok()?
        .json::<FingerprintResponse>()
        .await
        .ok()?;
    let mod_id = response.data.exact_matches.first()?.file.mod_id;
    let project = client
        .get(format!("https://api.curseforge.com/v1/mods/{mod_id}"))
        .header("x-api-key", key)
        .send()
        .await
        .ok()?
        .json::<ModResponse>()
        .await
        .ok()?;
    let expected = match kind {
        InstanceContentKind::Mods => 6,
        InstanceContentKind::ResourcePacks => 12,
        InstanceContentKind::ShaderPacks => 6552,
    };
    (project.data.class_id == Some(expected)).then_some(ContentMetadata {
        title: project.data.name,
        version: response
            .data
            .exact_matches
            .first()
            .and_then(|item| item.file.display_name.clone()),
        icon_url: project.data.logo.and_then(|logo| logo.url),
    })
}

pub(super) async fn fetch_curseforge_metadata(
    client: &Client,
    kind: InstanceContentKind,
    query: &str,
) -> Option<ContentMetadata> {
    let key = std::env::var("LICHT_CURSEFORGE_API_KEY").ok()?;
    let class_id = match kind {
        InstanceContentKind::Mods => 6,
        InstanceContentKind::ResourcePacks => 12,
        InstanceContentKind::ShaderPacks => 6552,
    };
    #[derive(Deserialize)]
    struct Hit {
        name: String,
        logo: Option<Logo>,
    }
    #[derive(Deserialize)]
    struct Logo {
        url: Option<String>,
    }
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Hit>,
    }
    let url = format!(
        "https://api.curseforge.com/v1/mods/search?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("gameId", "432")
            .append_pair("classId", &class_id.to_string())
            .append_pair("searchFilter", query)
            .append_pair("pageSize", "10")
            .finish()
    );
    let response = client
        .get(url)
        .header("x-api-key", key)
        .send()
        .await
        .ok()?
        .json::<Response>()
        .await
        .ok()?;
    response.data.into_iter().next().map(|hit| ContentMetadata {
        title: hit.name,
        version: None,
        icon_url: hit.logo.and_then(|logo| logo.url),
    })
}
