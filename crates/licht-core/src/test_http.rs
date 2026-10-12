//! Local HTTP stubs for unit tests.
//!
//! On Windows, many short-lived `127.0.0.1` sockets in parallel tokio tests can
//! fail with WSAEADDRINUSE (10048) when picking an ephemeral client port. Tests
//! that hit the network loopback should serialize on [`LOCAL_HTTP`] and retry.

use std::sync::LazyLock;
use std::time::Duration;

use tokio::sync::{Mutex, MutexGuard};

/// Serializes loopback stub servers so parallel unit tests do not exhaust ports.
static LOCAL_HTTP: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) async fn lock_local_http() -> MutexGuard<'static, ()> {
    LOCAL_HTTP.lock().await
}

pub(crate) fn error_chain(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(inner) = source {
        message.push_str(": ");
        message.push_str(&inner.to_string());
        source = inner.source();
    }
    message
}

pub(crate) fn is_transient_local_connect(error: &reqwest::Error) -> bool {
    let message = error_chain(error).to_ascii_lowercase();
    message.contains("10048")
        || message.contains("addrinuse")
        || message.contains("address already in use")
        || message.contains("normalmente é permitida apenas uma utilização")
}

/// Serves one response body and returns the request URL.
pub(crate) async fn serve_once(response: &str) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("address");
    let response = response.to_string();
    let handle = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return String::new();
        };
        let mut incoming = Vec::new();
        let mut buffer = [0_u8; 256];
        while let Ok(read) = socket.read(&mut buffer).await {
            if read == 0 {
                break;
            }
            incoming.extend_from_slice(&buffer[..read]);
            if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let _ = socket.write_all(response.as_bytes()).await;
        String::from_utf8_lossy(&incoming).into_owned()
    });
    (format!("http://{address}/file.jar"), handle)
}

/// Runs `request` after spinning a fresh stub, retrying transient Windows connect errors.
pub(crate) async fn with_stub_retries<F, Fut>(
    response: &str,
    mut request: F,
) -> Result<reqwest::Response, reqwest::Error>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let mut last_error = None;
    for attempt in 0..8 {
        let (url, _handle) = serve_once(response).await;
        match request(url).await {
            Ok(response) => return Ok(response),
            Err(error) if is_transient_local_connect(&error) && attempt + 1 < 8 => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.expect("retry loop kept a connect error"))
}
