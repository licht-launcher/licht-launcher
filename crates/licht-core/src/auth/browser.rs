use std::collections::HashMap;
use std::io;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use url::Url;

use crate::auth::client_id::msa_client_id;
use crate::auth::endpoints::{MSA_AUTHORIZE_URL, MSA_SCOPE, MSA_TOKEN_URL};
use crate::auth::parse::parse_token_response;
use crate::auth::types::TokenSuccess;
use crate::{CoreError, Result};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
const SUCCESS_BODY: &str = concat!(
    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Licht Launcher</title></head>",
    "<body><p>Sign-in complete. You can close this window and return to Licht Launcher.</p></body></html>"
);

struct Pkce {
    verifier: String,
    challenge: String,
}

fn random_urlsafe(byte_len: usize) -> Result<String> {
    let mut bytes = vec![0u8; byte_len];
    getrandom::fill(&mut bytes).map_err(|error| {
        CoreError::Io(io::Error::other(format!(
            "random bytes unavailable: {error}"
        )))
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn pkce_pair() -> Result<Pkce> {
    let verifier = random_urlsafe(32)?;
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    Ok(Pkce {
        verifier,
        challenge,
    })
}

fn build_authorize_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> Result<String> {
    let mut url = Url::parse(MSA_AUTHORIZE_URL).map_err(|_| CoreError::AuthToken)?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("client_id", client_id);
        query.append_pair("response_type", "code");
        query.append_pair("redirect_uri", redirect_uri);
        query.append_pair("response_mode", "query");
        // Adding an account must not silently reuse the browser's existing session.
        query.append_pair("prompt", "select_account");
        query.append_pair("scope", MSA_SCOPE);
        query.append_pair("state", state);
        query.append_pair("code_challenge", code_challenge);
        query.append_pair("code_challenge_method", "S256");
    }
    Ok(url.into())
}

/// Opens the system browser, waits for the localhost redirect, exchanges the code.
pub async fn browser_login_msa<F>(client: &Client, open_url: F) -> Result<TokenSuccess>
where
    F: FnOnce(&str) -> std::result::Result<(), String>,
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    // Microsoft accepts any port for http://localhost on public clients.
    let redirect_uri = format!("http://localhost:{port}/");
    let pkce = pkce_pair()?;
    let state = random_urlsafe(16)?;
    let client_id = msa_client_id()?;
    let authorize_url = build_authorize_url(&client_id, &redirect_uri, &state, &pkce.challenge)?;

    open_url(&authorize_url).map_err(|_| CoreError::AuthBrowserOpen)?;

    let code = wait_for_auth_code(listener, &state).await?;
    exchange_authorization_code(client, &code, &redirect_uri, &pkce.verifier).await
}

async fn exchange_authorization_code(
    client: &Client,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenSuccess> {
    let client_id = msa_client_id()?;
    let response = client
        .post(MSA_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    parse_token_response(status, &body)
}

async fn wait_for_auth_code(listener: TcpListener, expected_state: &str) -> Result<String> {
    let deadline = tokio::time::Instant::now() + LOGIN_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(CoreError::AuthBrowserTimeout);
        }
        let (mut socket, _) = match tokio::time::timeout(remaining, listener.accept()).await {
            Ok(Ok(accepted)) => accepted,
            Ok(Err(error)) => return Err(CoreError::Io(error)),
            Err(_) => return Err(CoreError::AuthBrowserTimeout),
        };

        let mut buffer = vec![0u8; 8192];
        let read =
            match tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                Ok(Ok(n)) => n,
                Ok(Err(_)) | Err(_) => continue,
            };
        let request = String::from_utf8_lossy(&buffer[..read]);
        let Some(target) = request_target(&request) else {
            let _ = write_response(&mut socket, 400, "Bad request").await;
            continue;
        };
        if target.starts_with("/favicon") {
            let _ = write_response(&mut socket, 404, "Not found").await;
            continue;
        }

        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
        let params = parse_query(query);

        if let Some(error) = params.get("error") {
            let _ = write_response(
                &mut socket,
                200,
                "Sign-in was cancelled. You can close this window.",
            )
            .await;
            if error == "access_denied" {
                return Err(CoreError::AuthDeclined);
            }
            return Err(CoreError::AuthOAuth {
                error: error.clone(),
                description: params.get("error_description").cloned().unwrap_or_default(),
            });
        }

        let Some(code) = params.get("code").cloned() else {
            let _ = write_response(&mut socket, 400, "Missing authorization code").await;
            continue;
        };
        match params.get("state") {
            Some(state) if state == expected_state => {}
            _ => {
                let _ = write_response(&mut socket, 400, "Invalid sign-in state").await;
                return Err(CoreError::AuthStateMismatch);
            }
        }

        let _ = write_response(&mut socket, 200, SUCCESS_BODY).await;
        return Ok(code);
    }
}

fn request_target(request: &str) -> Option<&str> {
    let line = request.lines().next()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    if method != "GET" && method != "HEAD" {
        return None;
    }
    Some(target)
}

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        map.insert(percent_decode(key), percent_decode(value));
    }
    map
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &input[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn write_response(socket: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(response.as_bytes()).await?;
    let _ = socket.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_handles_plus_and_hex() {
        assert_eq!(percent_decode("a+b%20c"), "a b c");
    }

    #[test]
    fn authorize_url_includes_pkce() {
        let url = build_authorize_url(
            "client",
            "http://localhost:1234/",
            "state-value",
            "challenge-value",
        )
        .expect("url");
        assert!(url.contains("code_challenge=challenge-value"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("prompt=select_account"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1234%2F"));
    }

    #[tokio::test]
    async fn localhost_redirect_returns_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let expected_state = "abc";
        let accept =
            tokio::spawn(async move { wait_for_auth_code(listener, expected_state).await });

        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request = format!(
            "GET /?code=the-code&state={expected_state} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("write request");
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.expect("read response");
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(response.starts_with("HTTP/1.1 200"));

        let code = accept.await.expect("join").expect("code");
        assert_eq!(code, "the-code");
    }
}
