use super::{BODY, SHA1};
use licht_core::{
    DownloadControl, DownloadOptions, DownloadProgress, DownloadTask, Retry, TransferLimits,
    download_all_with_control, download_file_with_control,
};
use licht_testkit::temp::TempDir;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

/// Upper bound for one step of a transfer test. Passing tests take milliseconds;
/// the margin only absorbs a loaded machine, such as CI or an antivirus scan.
const STEP_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn transfer(
    url: &str,
    destination: &Destination,
    control: &DownloadControl,
    attempts: u32,
) -> tokio::task::JoinHandle<licht_core::Result<()>> {
    let (url, path, control) = (url.to_string(), destination.path.clone(), control.clone());
    tokio::spawn(async move {
        download_file_with_control(
            &reqwest::Client::new(),
            &url,
            &path,
            SHA1,
            retry(attempts),
            &control,
        )
        .await
    })
}
pub(super) fn options() -> DownloadOptions {
    DownloadOptions {
        limits: Arc::new(TransferLimits::new()),
        download_concurrency: 1,
        write_concurrency: 1,
        retry: retry(1),
    }
}
pub(super) fn queue(
    url: &str,
    destination: &Destination,
    control: &DownloadControl,
    options: DownloadOptions,
) -> (
    tokio::task::JoinHandle<licht_core::Result<()>>,
    mpsc::Receiver<DownloadProgress>,
) {
    let task = DownloadTask {
        url: url.to_string(),
        destination: destination.path.clone(),
        sha1: SHA1.to_string(),
        size: 11,
    };
    let control = control.clone();
    let (progress, incoming) = mpsc::channel(64);
    let running = tokio::spawn(async move {
        download_all_with_control(
            &reqwest::Client::new(),
            &[task],
            &options,
            &control,
            progress,
        )
        .await
    });
    (running, incoming)
}
pub(super) async fn finish(
    running: tokio::task::JoinHandle<licht_core::Result<()>>,
) -> licht_core::Result<()> {
    tokio::time::timeout(STEP_TIMEOUT, running)
        .await
        .unwrap_or_else(|_| panic!("transfer did not finish within {STEP_TIMEOUT:?}"))
        .expect("join")
}
pub(super) async fn last_progress(
    progress: &mut mpsc::Receiver<DownloadProgress>,
) -> DownloadProgress {
    let mut last = None;
    while let Some(event) = progress.recv().await {
        last = Some(event);
    }
    last.expect("progress event")
}
pub(super) fn retry(attempts: u32) -> Retry {
    Retry {
        attempts,
        initial_backoff: Duration::from_millis(1),
    }
}

pub(super) struct Destination {
    pub(super) path: PathBuf,
    _directory: TempDir,
}
impl Destination {
    pub(super) fn new(name: &str) -> Self {
        let directory = TempDir::new(&format!("resume-{name}"));
        Self {
            path: directory.path().join("file.jar"),
            _directory: directory,
        }
    }
    pub(super) fn part(&self) -> PathBuf {
        self.path.with_file_name("file.jar.part")
    }
    pub(super) async fn wait_for_partial(&self, length: u64) {
        let part = self.part();
        let reached = tokio::time::timeout(STEP_TIMEOUT, async {
            loop {
                if std::fs::metadata(&part).is_ok_and(|metadata| metadata.len() == length) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        if reached.is_err() {
            let current = std::fs::metadata(&part).map_or_else(
                |_| "missing".to_string(),
                |metadata| format!("{} bytes", metadata.len()),
            );
            panic!(
                "partial file did not reach {length} bytes within {STEP_TIMEOUT:?} (now {current})"
            );
        }
    }
}

pub(super) struct Request {
    pub(super) headers: String,
    reply: oneshot::Sender<Reply>,
}
impl Request {
    pub(super) fn respond(self, reply: Reply) {
        self.reply
            .send(reply)
            .unwrap_or_else(|_| panic!("request must still be waiting"));
    }
}
pub(super) struct Reply {
    bytes: Vec<u8>,
    pub(super) tail: Option<oneshot::Receiver<Vec<u8>>>,
}
impl Reply {
    pub(super) fn full() -> Self {
        Self::new(200, "", BODY, BODY.len())
    }
    pub(super) fn suffix() -> Self {
        Self::new(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6)
    }
    pub(super) fn new(status: u16, headers: &str, body: &[u8], length: usize) -> Self {
        let mut bytes = format!("HTTP/1.1 {status} Response\r\nContent-Length: {length}\r\n{headers}Connection: close\r\n\r\n").into_bytes();
        bytes.extend_from_slice(body);
        Self { bytes, tail: None }
    }
}
pub(super) struct Server {
    pub(super) url: String,
    pub(super) incoming: mpsc::UnboundedReceiver<Request>,
    received: usize,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    pub(super) async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("http://{}/file", listener.local_addr().expect("address"));
        let (requests, incoming) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let requests = requests.clone();
                tokio::spawn(async move {
                    let mut headers = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = socket.read(&mut buffer).await.expect("request headers");
                        if count == 0 {
                            return;
                        }
                        headers.extend_from_slice(&buffer[..count]);
                        if headers.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let (reply, response) = oneshot::channel();
                    requests
                        .send(Request {
                            headers: String::from_utf8(headers).expect("headers"),
                            reply,
                        })
                        .expect("request receiver");
                    let reply = response.await.expect("response");
                    if socket.write_all(&reply.bytes).await.is_err() {
                        return;
                    }
                    if let Some(tail) = reply.tail
                        && let Ok(bytes) = tail.await
                    {
                        // The client may have intentionally paused or cancelled its connection.
                        let _ = socket.write_all(&bytes).await;
                    }
                });
            }
        });
        Self {
            url,
            incoming,
            received: 0,
            task,
        }
    }
    pub(super) async fn request(&mut self) -> Request {
        self.request_within(STEP_TIMEOUT).await
    }
    /// Waits for the next request; a timeout names which request never arrived.
    pub(super) async fn request_within(&mut self, limit: Duration) -> Request {
        let expected = self.received + 1;
        let request = tokio::time::timeout(limit, self.incoming.recv())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "request {expected} did not reach the test server within {limit:?} ({} received so far)",
                    self.received
                )
            })
            .expect("server request");
        self.received = expected;
        request
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
