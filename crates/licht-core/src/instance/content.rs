use std::collections::BTreeMap;

use futures_util::{StreamExt, stream};
use reqwest::Client;

use crate::store::repositories::content_fingerprints::ContentFingerprint;
use crate::{DatabaseClient, Result, SharedCache};

mod files;
mod fingerprints;
mod metadata;
mod providers;
mod types;

pub use files::{list_instance_content, remove_instance_content, set_instance_content_enabled};
pub use types::{InstanceContentEntry, InstanceContentKind, InstanceContentUpdate};

use files::instance_root;
use metadata::resolve_content_entry;

const CONTENT_METADATA_CONCURRENCY: usize = 4;

pub async fn list_instance_content_with_metadata(
    cache: &SharedCache,
    database: &DatabaseClient,
    client: &Client,
    folder: &str,
    refresh: bool,
) -> Result<Vec<InstanceContentEntry>> {
    list_instance_content_progressively(cache, database, client, folder, refresh, |_| Ok(())).await
}

pub async fn list_instance_content_progressively(
    cache: &SharedCache,
    database: &DatabaseClient,
    client: &Client,
    folder: &str,
    refresh: bool,
    mut publish: impl FnMut(&[InstanceContentEntry]) -> Result<()>,
) -> Result<Vec<InstanceContentEntry>> {
    let root = instance_root(cache, folder)?;
    let mut entries = list_instance_content(cache, folder)?;
    publish(&entries)?;
    let last_published = std::time::Instant::now();
    let previous = database.content_fingerprints(cache, folder).await?;
    let known: BTreeMap<_, _> = previous
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect();
    let lookups = entries
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            resolve_content_entry(cache, client, &root, &known, index, entry, refresh)
        });
    let fingerprints =
        enrich_content_entries(&mut entries, lookups, &mut publish, last_published).await?;
    database
        .sync_content_fingerprints(cache, folder, previous, fingerprints)
        .await?;
    Ok(entries)
}

struct ContentResolution {
    index: usize,
    entry: InstanceContentEntry,
    fingerprint: ContentFingerprint,
}

async fn enrich_content_entries<F>(
    entries: &mut [InstanceContentEntry],
    lookups: impl Iterator<Item = F>,
    publish: &mut impl FnMut(&[InstanceContentEntry]) -> Result<()>,
    mut last_published: std::time::Instant,
) -> Result<Vec<ContentFingerprint>>
where
    F: std::future::Future<Output = Result<ContentResolution>>,
{
    let mut pending =
        Box::pin(stream::iter(lookups).buffer_unordered(CONTENT_METADATA_CONCURRENCY));
    let mut fingerprints = Vec::with_capacity(entries.len());
    let mut batch = Vec::new();
    while let Some(result) = pending.next().await {
        let resolved = result?;
        entries[resolved.index] = resolved.entry.clone();
        batch.push(resolved.entry);
        fingerprints.push(resolved.fingerprint);
        if publish_content_batch(
            &batch,
            &mut last_published,
            std::time::Instant::now(),
            publish,
        )? {
            batch.clear();
        }
    }
    Ok(fingerprints)
}

fn publish_content_batch(
    entries: &[InstanceContentEntry],
    last_published: &mut std::time::Instant,
    now: std::time::Instant,
    publish: &mut impl FnMut(&[InstanceContentEntry]) -> Result<()>,
) -> Result<bool> {
    if now.duration_since(*last_published) < std::time::Duration::from_millis(100) {
        return Ok(false);
    }
    publish(entries)?;
    *last_published = now;
    Ok(true)
}

#[cfg(test)]
mod tests;
