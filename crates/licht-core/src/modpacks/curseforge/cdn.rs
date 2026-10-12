use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;

use crate::modpacks::secure_download::{
    allowlisted_download_client_with_headers, download_url_allowed,
};
use crate::{CoreError, Result};

/// Hosts the CurseForge API returns for file bytes.
///
/// Match is exact and HTTPS-only, including every redirect. `cdn.modrinth.com`
/// is absent on purpose: the `.mrpack` allowlist stays Modrinth-only, so a
/// forgecdn URL inside an `.mrpack` is still refused there.
pub(super) const CURSEFORGE_CDN_HOSTS: &[&str] = &[
    "edge.forgecdn.net",
    "mediafilez.forgecdn.net",
    "media.forgecdn.net",
];

/// Returns the URL without any `api-key` query parameter, or an error when the
/// host is not a CurseForge CDN host.
pub(super) fn canonical_download_url(raw: &str) -> Result<String> {
    if !download_url_allowed(raw, CURSEFORGE_CDN_HOSTS) {
        return Err(CoreError::CurseforgeDownloadHost);
    }
    let mut url = Url::parse(raw).map_err(|_| CoreError::CurseforgeDownloadHost)?;
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            !key.eq_ignore_ascii_case("api-key") && !key.eq_ignore_ascii_case("api_key")
        })
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    if !kept.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in &kept {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

/// First allowed URL of an index entry, with the `api-key` query stripped.
pub(super) fn select_cdn_url(urls: &[String]) -> Result<String> {
    if urls.is_empty() {
        return Err(CoreError::ModpackArchive);
    }
    for url in urls {
        if download_url_allowed(url, CURSEFORGE_CDN_HOSTS) {
            return canonical_download_url(url);
        }
    }
    Err(CoreError::CurseforgeDownloadHost)
}

/// Client that sends `x-api-key` on every request and follows a redirect only
/// when it stays on a CurseForge CDN host.
pub(super) fn cdn_client(key: &str) -> Result<Client> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(key).map_err(|_| CoreError::ModpackMeta {
        detail: "curseforge_api_key_invalid".into(),
    })?;
    let _ = headers.insert(HeaderName::from_static("x-api-key"), value);
    allowlisted_download_client_with_headers(
        CURSEFORGE_CDN_HOSTS,
        headers,
        "modpack redirect target is not allowed",
        "too many modpack redirects",
    )
}

pub(super) async fn fetch_cdn_bytes(key: &str, url: &str) -> Result<Vec<u8>> {
    let url = canonical_download_url(url)?;
    let client = cdn_client(key)?;
    let response = client.get(url).send().await?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(CoreError::ModpackMeta {
            detail: format!("curseforge_cdn_http_{status}"),
        });
    }
    Ok(response.bytes().await?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpacks::secure_download::{MAX_REDIRECTS, RedirectHop, redirect_hop};

    #[test]
    fn only_forgecdn_hosts_are_allowed() {
        for url in [
            "https://edge.forgecdn.net/files/1/2/a.jar",
            "https://MEDIAFILEZ.FORGECDN.NET/files/1/2/a.jar",
            "https://media.forgecdn.net/files/1/2/a.jar",
        ] {
            assert!(
                matches!(
                    redirect_hop(url, MAX_REDIRECTS, CURSEFORGE_CDN_HOSTS),
                    RedirectHop::Follow
                ),
                "{url}"
            );
        }
        for url in [
            "https://cdn.modrinth.com/data/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
            "https://objects.githubusercontent.com/a.jar",
            "https://edge.forgecdn.net.evil.example/a.jar",
            "https://evil.edge.forgecdn.net/a.jar",
        ] {
            assert!(
                matches!(
                    redirect_hop(url, 1, CURSEFORGE_CDN_HOSTS),
                    RedirectHop::Blocked
                ),
                "{url}"
            );
        }
    }

    #[test]
    fn canonical_url_strips_the_api_key_and_keeps_other_params() {
        let url = canonical_download_url(
            "https://edge.forgecdn.net/files/1/2/a.jar?api-key=secret&API_KEY=other&v=3",
        )
        .expect("url");
        assert!(!url.contains("secret"));
        assert!(!url.contains("other"));
        assert!(url.ends_with("/files/1/2/a.jar?v=3"), "{url}");
        let bare = canonical_download_url("https://edge.forgecdn.net/files/1/2/a.jar?api-key=x")
            .expect("url");
        assert_eq!(bare, "https://edge.forgecdn.net/files/1/2/a.jar");
    }

    #[test]
    fn canonical_url_rejects_hosts_outside_the_curseforge_cdn() {
        for url in [
            "https://cdn.modrinth.com/data/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
            "http://edge.forgecdn.net/files/1/2/a.jar",
        ] {
            let error = canonical_download_url(url).expect_err(url);
            assert!(matches!(error, CoreError::CurseforgeDownloadHost), "{url}");
        }
    }

    #[test]
    fn selects_the_first_cdn_url_and_strips_its_key() {
        let urls = vec![
            "https://github.com/a/b/releases/download/v/a.jar".to_string(),
            "https://edge.forgecdn.net/files/1/2/a.jar?api-key=secret".to_string(),
        ];
        let url = select_cdn_url(&urls).expect("url");
        assert_eq!(url, "https://edge.forgecdn.net/files/1/2/a.jar");
        assert!(matches!(
            select_cdn_url(&urls[..1]).unwrap_err(),
            CoreError::CurseforgeDownloadHost
        ));
        assert!(matches!(
            select_cdn_url(&[]).unwrap_err(),
            CoreError::ModpackArchive
        ));
    }

    #[test]
    fn an_api_key_that_is_not_a_header_value_is_refused() {
        let error = cdn_client("bad\nkey").unwrap_err();
        match error {
            CoreError::ModpackMeta { detail } => assert_eq!(detail, "curseforge_api_key_invalid"),
            other => panic!("expected an invalid key, got {other}"),
        }
    }

    #[tokio::test]
    async fn the_client_sends_the_api_key_header() {
        use crate::test_http::{lock_local_http, serve_once};

        let _guard = lock_local_http().await;
        let (url, request) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
        let client = cdn_client("test-key").expect("client");
        client.get(&url).send().await.expect("response");
        let request = request.await.expect("request").to_ascii_lowercase();
        assert!(request.contains("x-api-key: test-key"), "{request}");
    }

    #[tokio::test]
    async fn a_live_redirect_to_a_disallowed_url_is_rejected() {
        use crate::test_http::{error_chain, lock_local_http, with_stub_retries};

        let _guard = lock_local_http().await;
        let client = cdn_client("test-key").expect("client");
        for location in [
            "https://evil.example/malware.jar",
            "http://edge.forgecdn.net/a.jar",
            "https://cdn.modrinth.com/a.jar",
            "https://github.com/a/b/releases/download/v/a.jar",
        ] {
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let error = with_stub_retries(&response, |url| {
                let client = client.clone();
                async move { client.get(&url).send().await }
            })
            .await
            .expect_err(location);
            assert!(error.is_redirect(), "{location}: {error}");
            let message = error_chain(&error);
            assert!(
                message.contains("modpack redirect target is not allowed"),
                "{location}: {message}"
            );
        }
    }
}
