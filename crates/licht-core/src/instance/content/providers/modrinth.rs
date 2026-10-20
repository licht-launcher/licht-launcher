use reqwest::Client;
use serde::Deserialize;

use super::{ContentMetadata, InstanceContentKind};

pub(super) async fn fetch_modrinth_fingerprint(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    #[derive(Deserialize)]
    struct Version {
        project_id: String,
        version_number: String,
    }
    #[derive(Deserialize)]
    struct Project {
        title: String,
        icon_url: Option<String>,
        project_type: String,
    }
    let version = client
        .get(format!(
            "https://api.modrinth.com/v2/version_file/{fingerprint}?algorithm=sha1"
        ))
        .send()
        .await
        .ok()?
        .json::<Version>()
        .await
        .ok()?;
    let project = client
        .get(format!(
            "https://api.modrinth.com/v2/project/{}",
            version.project_id
        ))
        .send()
        .await
        .ok()?
        .json::<Project>()
        .await
        .ok()?;
    let expected = match kind {
        InstanceContentKind::Mods => "mod",
        InstanceContentKind::ResourcePacks => "resourcepack",
        InstanceContentKind::ShaderPacks => "shader",
    };
    (project.project_type == expected).then_some(ContentMetadata {
        title: project.title,
        version: Some(version.version_number),
        icon_url: project.icon_url,
    })
}

pub(super) async fn fetch_modrinth_metadata(
    client: &Client,
    kind: InstanceContentKind,
    query: &str,
) -> Option<ContentMetadata> {
    #[derive(Deserialize)]
    struct Hit {
        title: String,
        icon_url: Option<String>,
        project_type: String,
    }
    #[derive(Deserialize)]
    struct Response {
        hits: Vec<Hit>,
    }
    let expected = match kind {
        InstanceContentKind::Mods => "mod",
        InstanceContentKind::ResourcePacks => "resourcepack",
        InstanceContentKind::ShaderPacks => "shader",
    };
    let url = format!(
        "https://api.modrinth.com/v2/search?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("query", query)
            .append_pair("limit", "10")
            .finish()
    );
    let response = client
        .get(url)
        .send()
        .await
        .ok()?
        .json::<Response>()
        .await
        .ok()?;
    response
        .hits
        .into_iter()
        .find(|hit| hit.project_type == expected)
        .map(|hit| ContentMetadata {
            title: hit.title,
            version: None,
            icon_url: hit.icon_url,
        })
}
