//! A short-lived in-memory cache for loader metadata responses.
//!
//! The same documents are asked for again and again while a user picks a
//! version (and again when the instance is created), and the Forge ones are
//! hundreds of kilobytes, so they are kept for a few minutes.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::Client;

use crate::Result;

const TTL: Duration = Duration::from_secs(600);

/// A response worth keeping: the status and the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CachedResponse {
    pub status: u16,
    pub body: String,
}

impl CachedResponse {
    /// Server errors and rate limits are retried on the next request.
    fn cacheable(&self) -> bool {
        (200..300).contains(&self.status) || self.status == 400 || self.status == 404
    }
}

#[derive(Default)]
struct TtlCache {
    entries: HashMap<String, (Instant, CachedResponse)>,
}

impl TtlCache {
    fn get(&self, key: &str, now: Instant) -> Option<CachedResponse> {
        let (stored_at, response) = self.entries.get(key)?;
        (now.duration_since(*stored_at) < TTL).then(|| response.clone())
    }

    fn insert(&mut self, key: &str, response: CachedResponse, now: Instant) {
        self.entries
            .retain(|_, (stored_at, _)| now.duration_since(*stored_at) < TTL);
        self.entries.insert(key.to_string(), (now, response));
    }
}

fn shared() -> &'static Mutex<TtlCache> {
    static CACHE: OnceLock<Mutex<TtlCache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// GETs `url`, reusing a recent answer when there is one.
pub(crate) async fn fetch_cached(client: &Client, url: &str) -> Result<CachedResponse> {
    if let Some(hit) = shared()
        .lock()
        .ok()
        .and_then(|cache| cache.get(url, Instant::now()))
    {
        return Ok(hit);
    }
    let response = client.get(url).send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    let fresh = CachedResponse { status, body };
    if fresh.cacheable()
        && let Ok(mut cache) = shared().lock()
    {
        cache.insert(url, fresh.clone(), Instant::now());
    }
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(body: &str) -> CachedResponse {
        CachedResponse {
            status: 200,
            body: body.into(),
        }
    }

    #[test]
    fn serves_an_answer_until_it_expires() {
        let start = Instant::now();
        let mut cache = TtlCache::default();
        cache.insert("a", ok("one"), start);

        assert_eq!(
            cache.get("a", start + Duration::from_secs(599)),
            Some(ok("one"))
        );
        assert_eq!(cache.get("a", start + TTL), None);
    }

    #[test]
    fn keeps_answers_apart_by_url() {
        let start = Instant::now();
        let mut cache = TtlCache::default();
        cache.insert("a", ok("one"), start);

        assert_eq!(cache.get("b", start), None);
    }

    #[test]
    fn drops_expired_answers_when_storing_a_new_one() {
        let start = Instant::now();
        let mut cache = TtlCache::default();
        cache.insert("old", ok("one"), start);
        cache.insert("new", ok("two"), start + TTL);

        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn caches_missing_versions_but_not_server_errors() {
        let status = |status| {
            CachedResponse {
                status,
                body: String::new(),
            }
            .cacheable()
        };

        assert!(status(200) && status(400) && status(404));
        assert!(!status(500) && !status(429));
    }
}
