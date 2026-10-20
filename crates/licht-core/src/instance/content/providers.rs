use reqwest::Client;

use super::InstanceContentKind;
use super::metadata::{ContentMetadata, fallback_title};

mod curseforge;
mod modrinth;

use curseforge::{fetch_curseforge_fingerprint, fetch_curseforge_metadata};
use modrinth::{fetch_modrinth_fingerprint, fetch_modrinth_metadata};

pub(super) async fn fetch_remote_metadata(
    client: &Client,
    kind: InstanceContentKind,
    name: &str,
) -> Option<ContentMetadata> {
    let query = fallback_title(name);
    first_metadata(
        fetch_modrinth_metadata(client, kind, &query),
        fetch_curseforge_metadata(client, kind, &query),
    )
    .await
}

pub(super) async fn fetch_fingerprint_metadata(
    client: &Client,
    kind: InstanceContentKind,
    fingerprint: &str,
) -> Option<ContentMetadata> {
    first_metadata(
        fetch_modrinth_fingerprint(client, kind, fingerprint),
        fetch_curseforge_fingerprint(client, kind, fingerprint),
    )
    .await
}

pub(super) async fn first_metadata(
    primary: impl std::future::Future<Output = Option<ContentMetadata>>,
    fallback: impl std::future::Future<Output = Option<ContentMetadata>>,
) -> Option<ContentMetadata> {
    match primary.await {
        Some(metadata) => Some(metadata),
        None => fallback.await,
    }
}
