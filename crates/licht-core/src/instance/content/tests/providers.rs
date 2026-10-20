use super::super::providers::first_metadata;
use super::*;

fn metadata(title: &str) -> ContentMetadata {
    ContentMetadata {
        title: title.to_string(),
        version: Some("1.0".to_string()),
        icon_url: None,
    }
}

#[tokio::test]
async fn metadata_hit_does_not_execute_the_fallback() {
    let primary = async { Some(metadata("Primary")) };
    let fallback = async {
        panic!("a metadata hit must not execute the fallback provider");
    };
    let result = first_metadata(primary, fallback).await.expect("metadata");
    assert_eq!(result.title, "Primary");
    assert_eq!(result.version.as_deref(), Some("1.0"));
}

#[tokio::test]
async fn metadata_miss_awaits_the_primary_before_trying_the_fallback() {
    let primary_finished = std::cell::Cell::new(false);
    let primary = async {
        tokio::task::yield_now().await;
        primary_finished.set(true);
        None
    };
    let fallback = async {
        assert!(primary_finished.get(), "primary lookup must finish first");
        Some(metadata("Fallback"))
    };
    let result = first_metadata(primary, fallback).await.expect("metadata");
    assert_eq!(result.title, "Fallback");
    assert_eq!(result.version.as_deref(), Some("1.0"));
}

#[tokio::test]
async fn metadata_miss_in_both_providers_preserves_the_missing_result() {
    let result = first_metadata(async { None }, async { None }).await;
    assert!(result.is_none());
}
