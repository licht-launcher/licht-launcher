use super::{BODY, SHA1};
use licht_core::{
    DownloadControl, DownloadOptions, DownloadProgress, DownloadTask, Retry, TransferLimits,
    download_all_with_control, download_file_with_control,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

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
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("transfer timeout")
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
}
impl Destination {
    pub(super) fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("licht-resume-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        Self {
            path: directory.join("file.jar"),
        }
    }
    pub(super) fn part(&self) -> PathBuf {
        self.path.with_file_name("file.jar.part")
    }
    pub(super) async fn wait_for_partial(&self, length: u64) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if std::fs::metadata(self.part()).is_ok_and(|metadata| metadata.len() == length) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("partial write timeout");
    }
}
impl Drop for Destination {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.path.parent().expect("directory")).expect("cleanup");
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
            task,
        }
    }
    pub(super) async fn request(&mut self) -> Request {
        tokio::time::timeout(Duration::from_secs(5), self.incoming.recv())
            .await
            .expect("request timeout")
            .expect("server request")
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
