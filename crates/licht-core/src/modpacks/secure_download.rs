use reqwest::Client;
use reqwest::header::HeaderMap;

use crate::{CoreError, Result};

/// Same cap as reqwest's default. The Modrinth spec asks for at least three.
pub(super) const MAX_REDIRECTS: usize = 10;

pub(super) enum RedirectHop {
    Follow,
    Blocked,
    TooMany,
}

/// HTTPS only, no explicit port, no credentials, and a domain host that matches
/// one of `hosts` exactly (case-insensitive).
pub(super) fn download_url_allowed(raw: &str, hosts: &[&str]) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https" || url.port().is_some() {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(url::Host::Domain(host)) = url.host() else {
        return false;
    };
    hosts
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
}

/// `previous_len` is how many URLs were already requested, including the first one.
pub(super) fn redirect_hop(next: &str, previous_len: usize, hosts: &[&str]) -> RedirectHop {
    if previous_len > MAX_REDIRECTS {
        return RedirectHop::TooMany;
    }
    if download_url_allowed(next, hosts) {
        RedirectHop::Follow
    } else {
        RedirectHop::Blocked
    }
}

/// Builds a client that follows a redirect only when that hop passes
/// [`download_url_allowed`] for `hosts`. The messages end up in the request error.
pub(super) fn allowlisted_download_client(
    hosts: &[&str],
    blocked_msg: &str,
    too_many_msg: &str,
) -> Result<Client> {
    allowlisted_download_client_with_headers(hosts, HeaderMap::new(), blocked_msg, too_many_msg)
}

/// Same as [`allowlisted_download_client`], plus `headers` sent on every request.
pub(super) fn allowlisted_download_client_with_headers(
    hosts: &[&str],
    headers: HeaderMap,
    blocked_msg: &str,
    too_many_msg: &str,
) -> Result<Client> {
    let hosts: Vec<String> = hosts.iter().map(|host| host.to_string()).collect();
    let blocked_msg = blocked_msg.to_string();
    let too_many_msg = too_many_msg.to_string();
    Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let allowed: Vec<&str> = hosts.iter().map(String::as_str).collect();
            match redirect_hop(attempt.url().as_str(), attempt.previous().len(), &allowed) {
                RedirectHop::Follow => attempt.follow(),
                RedirectHop::Blocked => attempt.error(blocked_msg.clone()),
                RedirectHop::TooMany => attempt.error(too_many_msg.clone()),
            }
        }))
        .build()
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: &[&str] = &["cdn.modrinth.com"];
    const BLOCKED: &str = "redirect target is not allowed";
    const TOO_MANY: &str = "too many redirects";

    #[test]
    fn url_rules_are_strict() {
        for url in [
            "https://cdn.modrinth.com/data/a.jar",
            "https://CDN.MODRINTH.COM/data/a.jar",
        ] {
            assert!(download_url_allowed(url, HOSTS), "{url}");
        }
        for url in [
            "http://cdn.modrinth.com/a.jar",
            "https://cdn.modrinth.com:444/a.jar",
            "https://user:secret@cdn.modrinth.com/a.jar",
            "https://cdn.modrinth.com.evil.com/a.jar",
            "https://evil.cdn.modrinth.com/a.jar",
            "https://1.2.3.4/a.jar",
            "https://github.com/user/repo/releases/download/v1/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar",
            "not a url",
        ] {
            assert!(!download_url_allowed(url, HOSTS), "{url}");
        }
    }

    #[test]
    fn hosts_come_from_the_caller() {
        let url = "https://edge.forgecdn.net/files/1/2/a.jar";
        assert!(!download_url_allowed(url, HOSTS));
        assert!(download_url_allowed(url, &["edge.forgecdn.net"]));
        assert!(!download_url_allowed(url, &[]));
    }

    #[test]
    fn follows_only_allowed_https_redirects() {
        for url in [
            "https://cdn.modrinth.com/data/a.jar",
            "https://CDN.MODRINTH.COM/data/a.jar",
        ] {
            assert!(
                matches!(redirect_hop(url, 1, HOSTS), RedirectHop::Follow),
                "{url}"
            );
            assert!(
                matches!(redirect_hop(url, MAX_REDIRECTS, HOSTS), RedirectHop::Follow),
                "{url}"
            );
        }
        assert!(matches!(
            redirect_hop("https://cdn.modrinth.com/a.jar", MAX_REDIRECTS + 1, HOSTS),
            RedirectHop::TooMany
        ));
        for url in [
            "http://cdn.modrinth.com/a.jar",
            "https://github.com/user/repo/releases/download/v1/a.jar",
            "https://raw.githubusercontent.com/user/repo/main/a.jar",
            "https://gitlab.com/user/repo/-/raw/main/a.jar",
            "https://objects.githubusercontent.com/a.jar",
            "https://release-assets.githubusercontent.com/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar",
            "https://mediafilez.forgecdn.net/files/1/2/a.jar",
            "https://evil.example/a.jar",
            "https://user:secret@cdn.modrinth.com/a.jar",
            "https://cdn.modrinth.com:444/a.jar",
        ] {
            assert!(
                matches!(redirect_hop(url, 1, HOSTS), RedirectHop::Blocked),
                "{url}"
            );
        }
    }

    #[test]
    fn stops_a_redirect_chain_at_the_first_disallowed_hop() {
        let hops = [
            "https://cdn.modrinth.com/a.jar",
            "https://CDN.MODRINTH.COM/b.jar",
            "https://edge.forgecdn.net/files/1/2/malware.jar",
        ];
        for (index, hop) in hops.iter().enumerate() {
            // The first previous entry is the URL that was just requested.
            let decision = redirect_hop(hop, index + 1, HOSTS);
            if index + 1 == hops.len() {
                assert!(matches!(decision, RedirectHop::Blocked), "{hop}");
            } else {
                assert!(matches!(decision, RedirectHop::Follow), "{hop}");
            }
        }
    }

    #[tokio::test]
    async fn a_live_redirect_to_a_disallowed_url_is_rejected() {
        use crate::test_http::{error_chain, lock_local_http, with_stub_retries};

        let _guard = lock_local_http().await;
        let client = allowlisted_download_client(HOSTS, BLOCKED, TOO_MANY).expect("client");
        for location in [
            "https://evil.example/malware.jar",
            "http://cdn.modrinth.com/a.jar",
            "https://github.com/user/repo/releases/download/v1/a.jar",
            "https://edge.forgecdn.net/files/1/2/a.jar",
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
            assert!(error.is_redirect(), "{location}: {}", error_chain(&error));
            let message = error_chain(&error);
            assert!(message.contains(BLOCKED), "{location}: {message}");
        }
    }
}
