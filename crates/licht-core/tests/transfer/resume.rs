use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use licht_core::{
    CoreError, DownloadChecksum, DownloadControl, DownloadOptions, DownloadProgress, DownloadState,
    DownloadTask, Retry, TransferLimits, VerifiedDownloadTask, download_all_with_control,
    download_file, download_file_with_control, download_verified_all,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

const BODY: &[u8] = b"hello world";
const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

#[tokio::test]
async fn every_published_checksum_can_resume_and_report_a_previously_unknown_size() {
    for (index, checksum) in [
        DownloadChecksum::Sha1(SHA1.into()),
        DownloadChecksum::Sha256("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".into()),
        DownloadChecksum::Sha512("309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f".into()),
        DownloadChecksum::Md5("5eb63bbbe01eeed093cb22bb8f5acdc3".into()),
    ].into_iter().enumerate() {
        let mut server = Server::start().await;
        let destination = Destination::new(&format!("checksum-{index}"));
        std::fs::write(destination.part(), b"hello").expect("partial");
        let task = VerifiedDownloadTask { url: server.url.clone(), destination: destination.path.clone(), checksum, size: 0 };
        let (progress, mut incoming) = mpsc::channel(64);
        let running = tokio::spawn(async move {
            download_verified_all(&reqwest::Client::new(), &[task], &options(), &DownloadControl::new(), progress).await
        });
        let request = server.request().await;
        assert_range(&request, 5);
        request.respond(Reply::suffix());
        finish(running).await.expect("verified suffix");
        let progress = last_progress(&mut incoming).await;
        assert_eq!((progress.bytes_done, progress.bytes_total, progress.finished), (11, 11, 1));
        assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
        assert!(!destination.part().exists());
    }
}

#[tokio::test]
async fn checksum_failures_keep_their_algorithm_and_never_publish_the_file() {
    for (index, checksum) in [
        DownloadChecksum::Sha256("0".repeat(64)),
        DownloadChecksum::Sha512("0".repeat(128)),
        DownloadChecksum::Md5("0".repeat(32)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut server = Server::start().await;
        let destination = Destination::new(&format!("bad-checksum-{index}"));
        let task = VerifiedDownloadTask {
            url: server.url.clone(),
            destination: destination.path.clone(),
            checksum: checksum.clone(),
            size: 11,
        };
        let (progress, _) = mpsc::channel(64);
        let running = tokio::spawn(async move {
            download_verified_all(
                &reqwest::Client::new(),
                &[task],
                &options(),
                &DownloadControl::new(),
                progress,
            )
            .await
        });
        server.request().await.respond(Reply::full());
        let error = finish(running).await.expect_err("checksum mismatch");
        assert!(match checksum {
            DownloadChecksum::Sha256(_) => matches!(error, CoreError::Sha256Mismatch { .. }),
            DownloadChecksum::Sha512(_) => matches!(error, CoreError::Sha512Mismatch { .. }),
            DownloadChecksum::Md5(_) => matches!(error, CoreError::Md5Mismatch { .. }),
            _ => false,
        });
        assert!(!destination.path.exists());
        assert!(!destination.part().exists());
    }
}

#[tokio::test]
async fn an_interrupted_transfer_keeps_bytes_and_a_new_call_resumes_them() {
    let mut server = Server::start().await;
    let destination = Destination::new("disconnected");
    let url = server.url.clone();
    let path = destination.path.clone();
    let first = tokio::spawn(async move {
        download_file(&reqwest::Client::new(), &url, &path, SHA1, retry(1)).await
    });
    server
        .request()
        .await
        .respond(Reply::new(200, "", b"hello", BODY.len()));
    first
        .await
        .expect("join")
        .expect_err("the truncated body must fail");
    assert_eq!(
        std::fs::read(destination.part()).expect("partial file"),
        b"hello"
    );
    assert!(!destination.path.exists());

    let url = server.url.clone();
    let path = destination.path.clone();
    let second = tokio::spawn(async move {
        download_file(&reqwest::Client::new(), &url, &path, SHA1, retry(1)).await
    });
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::suffix());
    second.await.expect("join").expect("resume");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
    assert!(!destination.part().exists());
}

#[tokio::test]
async fn a_retry_resumes_the_partial_instead_of_counting_its_bytes_twice() {
    let mut server = Server::start().await;
    let destination = Destination::new("retry-progress");
    let mut options = options();
    options.retry = retry(2);
    let (running, mut progress) =
        queue(&server.url, &destination, &DownloadControl::new(), options);
    server
        .request()
        .await
        .respond(Reply::new(200, "", b"hello", 11));
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::suffix());
    finish(running).await.expect("retry");
    let last = last_progress(&mut progress).await;
    assert_eq!((last.finished, last.failed), (1, 0));
    assert_eq!((last.bytes_done, last.bytes_total), (11, 11));
}

#[tokio::test]
async fn a_server_ignoring_range_replaces_the_partial_instead_of_appending() {
    let mut server = Server::start().await;
    let destination = Destination::new("ignores-range");
    std::fs::write(destination.part(), b"hello").expect("partial");
    let running = transfer(&server.url, &destination, &DownloadControl::new(), 1);
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::new(200, "", BODY, 11));
    finish(running).await.expect("full response");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
}

#[tokio::test]
async fn an_unsatisfiable_range_restarts_with_a_full_request() {
    let mut server = Server::start().await;
    let destination = Destination::new("range-416");
    std::fs::write(destination.part(), b"oversized stale data").expect("partial");
    let running = transfer(&server.url, &destination, &DownloadControl::new(), 1);
    let request = server.request().await;
    assert_range(&request, 20);
    request.respond(Reply::new(416, "Content-Range: bytes */11\r\n", b"", 0));
    let request = server.request().await;
    assert!(!request.headers.to_ascii_lowercase().contains("range:"));
    request.respond(Reply::new(200, "", BODY, 11));
    finish(running).await.expect("restart");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
}

#[tokio::test]
async fn invalid_partial_responses_are_rejected_before_publishing() {
    for (index, headers) in [
        "",
        "Content-Range: bytes 4-10/11\r\n",
        "Content-Range: bytes 5-9/11\r\n",
        "Content-Range: bytes 5-10/*\r\n",
        "Content-Range: bytes 5-10/11\r\nContent-Encoding: gzip\r\n",
        "Content-Range: bytes 5-11/12\r\n",
    ]
    .iter()
    .enumerate()
    {
        let mut server = Server::start().await;
        let destination = Destination::new(&format!("invalid-range-{index}"));
        std::fs::write(destination.part(), b"hello").expect("partial");
        let running = transfer(&server.url, &destination, &DownloadControl::new(), 1);
        let request = server.request().await;
        assert_range(&request, 5);
        request.respond(Reply::new(206, headers, b" world", 6));
        assert!(matches!(
            finish(running).await,
            Err(CoreError::DownloadResponse { .. })
        ));
        assert!(!destination.path.exists());
        assert!(!destination.part().exists());
    }
}

#[tokio::test]
async fn a_bad_partial_checksum_is_discarded_before_retrying_the_whole_file() {
    let mut server = Server::start().await;
    let destination = Destination::new("bad-prefix");
    std::fs::write(&destination.path, b"previous destination").expect("existing file");
    std::fs::write(destination.part(), b"xxxxx").expect("partial");
    let running = transfer(&server.url, &destination, &DownloadControl::new(), 2);
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::suffix());
    let request = server.request().await;
    assert!(!request.headers.to_ascii_lowercase().contains("range:"));
    assert!(!destination.part().exists());
    assert_eq!(
        std::fs::read(&destination.path).expect("existing file"),
        b"previous destination"
    );
    request.respond(Reply::new(200, "", BODY, 11));
    finish(running).await.expect("verified retry");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
}

#[tokio::test]
async fn a_download_without_a_checksum_does_not_reuse_unverified_partial_bytes() {
    let mut server = Server::start().await;
    let destination = Destination::new("unverified-partial");
    std::fs::write(destination.part(), b"unverified prefix").expect("partial");
    let (url, path) = (server.url.clone(), destination.path.clone());
    let running = tokio::spawn(async move {
        download_file(&reqwest::Client::new(), &url, &path, "", retry(1)).await
    });
    let request = server.request().await;
    assert!(!request.headers.to_ascii_lowercase().contains("range:"));
    request.respond(Reply::full());
    finish(running).await.expect("full unverified transfer");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
    assert!(!destination.part().exists());
}

#[tokio::test]
async fn a_complete_verified_partial_is_published_without_another_request() {
    let server = Server::start().await;
    let destination = Destination::new("complete-partial");
    std::fs::write(destination.part(), BODY).expect("partial");
    let running = transfer(&server.url, &destination, &DownloadControl::new(), 1);
    finish(running).await.expect("publish");
    assert!(server.incoming.is_empty());
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
    assert!(!destination.part().exists());
}

#[tokio::test]
async fn a_pause_releases_the_network_slot_and_resumes_from_disk() {
    let mut server = Server::start().await;
    let destination = Destination::new("paused");
    let other = Destination::new("unblocked");
    let control = DownloadControl::new();
    let options = options();
    let (running, mut progress) = queue(&server.url, &destination, &control, options.clone());
    let mut reply = Reply::new(200, "", b"hello", 11);
    let (tail, incoming_tail) = oneshot::channel();
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    control.pause();
    assert_eq!(control.state(), DownloadState::Paused);

    let (other_running, _) = queue(
        &format!("{}?other", server.url),
        &other,
        &DownloadControl::new(),
        options,
    );
    let request = server.request().await;
    assert!(request.headers.starts_with("GET /file?other "));
    request.respond(Reply::new(200, "", BODY, 11));
    finish(other_running).await.expect("other transfer");
    assert!(!running.is_finished());
    assert_eq!(
        std::fs::read(destination.part()).expect("partial"),
        b"hello"
    );

    control.resume();
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::suffix());
    finish(running).await.expect("resumed transfer");
    drop(tail);
    let last = last_progress(&mut progress).await;
    assert_eq!((last.finished, last.bytes_done), (1, 11));
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
}

#[tokio::test]
async fn cancelling_an_active_transfer_removes_its_partial_and_preserves_the_destination() {
    let mut server = Server::start().await;
    let destination = Destination::new("cancel-active");
    std::fs::write(&destination.path, b"previous destination").expect("existing destination");
    let control = DownloadControl::new();
    let running = transfer(&server.url, &destination, &control, 1);
    let (tail, incoming_tail) = oneshot::channel();
    let mut reply = Reply::new(200, "", b"hello", 11);
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    control.pause();
    tokio::task::yield_now().await;
    control.cancel();
    control.resume();
    assert_eq!(control.state(), DownloadState::Cancelled);
    assert!(matches!(
        finish(running).await,
        Err(CoreError::DownloadCancelled)
    ));
    assert!(!destination.part().exists());
    assert_eq!(
        std::fs::read(&destination.path).expect("existing file"),
        b"previous destination"
    );
    drop(tail);
}

#[tokio::test]
async fn a_paused_control_does_not_start_network_requests_until_resumed() {
    let mut server = Server::start().await;
    let destination = Destination::new("paused-before-start");
    let control = DownloadControl::new();
    control.pause();
    let running = transfer(&server.url, &destination, &control, 1);
    tokio::task::yield_now().await;
    assert!(server.incoming.is_empty());
    assert!(!running.is_finished());
    assert!(!destination.part().exists());
    control.resume();
    server.request().await.respond(Reply::full());
    finish(running).await.expect("resumed initial transfer");
}

#[tokio::test]
async fn cancelling_the_public_verified_queue_removes_its_unshared_partial() {
    let mut server = Server::start().await;
    let destination = Destination::new("cancel-verified-queue");
    let control = DownloadControl::new();
    let worker_control = control.clone();
    let task = VerifiedDownloadTask {
        url: server.url.clone(),
        destination: destination.path.clone(),
        checksum: DownloadChecksum::Sha256(
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".into(),
        ),
        size: 11,
    };
    let (progress, _) = mpsc::channel(64);
    let running = tokio::spawn(async move {
        download_verified_all(
            &reqwest::Client::new(),
            &[task],
            &options(),
            &worker_control,
            progress,
        )
        .await
    });
    let (tail, incoming_tail) = oneshot::channel();
    let mut reply = Reply::new(200, "", b"hello", 11);
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    control.cancel();
    assert!(matches!(
        finish(running).await,
        Err(CoreError::DownloadCancelled)
    ));
    assert!(!destination.part().exists());
    assert!(!destination.path.exists());
    drop(tail);
}

#[tokio::test]
async fn cancelling_a_waiter_does_not_remove_another_transfers_partial() {
    let mut server = Server::start().await;
    let destination = Destination::new("shared-cancel");
    let first = transfer(&server.url, &destination, &DownloadControl::new(), 1);
    let (tail, incoming_tail) = oneshot::channel();
    let mut reply = Reply::new(200, "", b"hello", 11);
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    let control = DownloadControl::new();
    let waiter = transfer(&server.url, &destination, &control, 1);
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());
    control.cancel();
    assert!(matches!(
        finish(waiter).await,
        Err(CoreError::DownloadCancelled)
    ));
    assert_eq!(
        std::fs::read(destination.part()).expect("shared partial"),
        b"hello"
    );
    tail.send(b" world".to_vec())
        .expect("finish first response");
    finish(first).await.expect("first transfer");
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
    assert!(server.incoming.is_empty());
}

#[tokio::test]
async fn dropping_a_queue_leaves_no_detached_transfer_and_allows_a_new_call() {
    let mut server = Server::start().await;
    let destination = Destination::new("abort-queue");
    let (running, _) = queue(
        &server.url,
        &destination,
        &DownloadControl::new(),
        options(),
    );
    let (tail, incoming_tail) = oneshot::channel();
    let mut reply = Reply::new(200, "", b"hello", 11);
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    running.abort();
    assert!(running.await.expect_err("aborted queue").is_cancelled());

    let restarted = transfer(&server.url, &destination, &DownloadControl::new(), 1);
    let request = server.request().await;
    assert_range(&request, 5);
    request.respond(Reply::suffix());
    finish(restarted).await.expect("restarted transfer");
    drop(tail);
    assert_eq!(std::fs::read(&destination.path).expect("file"), BODY);
}

fn assert_range(request: &Request, offset: u64) {
    let headers = request.headers.to_ascii_lowercase();
    assert!(
        headers.contains(&format!("range: bytes={offset}-\r\n")),
        "{headers}"
    );
    assert!(headers.contains("accept-encoding: identity\r\n"));
}

#[tokio::test]
async fn cancelling_a_queue_stops_waiting_files_without_counting_them_as_failures() {
    let mut server = Server::start().await;
    let destination = Destination::new("cancel-queue");
    let waiting = Destination::new("cancel-queued-file");
    let cached = Destination::new("cancel-cached-file");
    std::fs::write(&cached.path, BODY).expect("cached file");
    let tasks: Vec<_> = [&destination, &waiting, &cached]
        .into_iter()
        .map(|destination| DownloadTask {
            url: server.url.clone(),
            destination: destination.path.clone(),
            sha1: SHA1.to_string(),
            size: 11,
        })
        .collect();
    let control = DownloadControl::new();
    let worker_control = control.clone();
    let (progress, mut incoming) = mpsc::channel(64);
    let running = tokio::spawn(async move {
        download_all_with_control(
            &reqwest::Client::new(),
            &tasks,
            &options(),
            &worker_control,
            progress,
        )
        .await
    });
    let (tail, incoming_tail) = oneshot::channel();
    let mut reply = Reply::new(200, "", b"hello", 11);
    reply.tail = Some(incoming_tail);
    server.request().await.respond(reply);
    destination.wait_for_partial(5).await;
    control.cancel();
    assert!(matches!(
        finish(running).await,
        Err(CoreError::DownloadCancelled)
    ));
    assert!(!destination.part().exists());
    assert!(!waiting.path.exists());
    assert!(!waiting.part().exists());
    assert_eq!(std::fs::read(&cached.path).expect("cached file"), BODY);
    assert!(server.incoming.is_empty());
    assert_eq!(last_progress(&mut incoming).await.failed, 0);
    drop(tail);
}

#[tokio::test]
async fn cancellation_interrupts_waiting_for_a_disk_write_slot() {
    let mut server = Server::start().await;
    let destination = Destination::new("cancel-write-wait");
    let control = DownloadControl::new();
    let options = options();
    let permit = options.limits.write.acquire(1).await;
    let (running, _) = queue(&server.url, &destination, &control, options.clone());
    server.request().await.respond(Reply::full());
    destination.wait_for_partial(0).await;
    control.cancel();
    assert!(matches!(
        finish(running).await,
        Err(CoreError::DownloadCancelled)
    ));
    assert!(!destination.path.exists());
    assert!(!destination.part().exists());
    drop(permit);
}

#[tokio::test]
async fn a_response_disagreeing_with_the_published_size_is_rejected() {
    let mut server = Server::start().await;
    let destination = Destination::new("published-size");
    let task = DownloadTask {
        url: server.url.clone(),
        destination: destination.path.clone(),
        sha1: SHA1.to_string(),
        size: 12,
    };
    let (progress, mut incoming) = mpsc::channel(64);
    let running = tokio::spawn(async move {
        download_all_with_control(
            &reqwest::Client::new(),
            &[task],
            &options(),
            &DownloadControl::new(),
            progress,
        )
        .await
    });
    server.request().await.respond(Reply::full());
    assert!(matches!(
        finish(running).await,
        Err(CoreError::DownloadResponse { .. })
    ));
    assert!(!destination.path.exists());
    assert!(!destination.part().exists());
    let last = last_progress(&mut incoming).await;
    assert_eq!((last.finished, last.failed, last.bytes_done), (0, 1, 0));
}

fn transfer(
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

fn options() -> DownloadOptions {
    DownloadOptions {
        limits: Arc::new(TransferLimits::new()),
        download_concurrency: 1,
        write_concurrency: 1,
        retry: retry(1),
    }
}

fn queue(
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

async fn finish(
    running: tokio::task::JoinHandle<licht_core::Result<()>>,
) -> licht_core::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("transfer timeout")
        .expect("join")
}

async fn last_progress(progress: &mut mpsc::Receiver<DownloadProgress>) -> DownloadProgress {
    let mut last = None;
    while let Some(event) = progress.recv().await {
        last = Some(event);
    }
    last.expect("progress event")
}

fn retry(attempts: u32) -> Retry {
    Retry {
        attempts,
        initial_backoff: Duration::from_millis(1),
    }
}

struct Destination {
    path: PathBuf,
}

impl Destination {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("licht-resume-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        Self {
            path: directory.join("file.jar"),
        }
    }

    fn part(&self) -> PathBuf {
        self.path.with_file_name("file.jar.part")
    }

    async fn wait_for_partial(&self, length: u64) {
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

struct Request {
    headers: String,
    reply: oneshot::Sender<Reply>,
}

impl Request {
    fn respond(self, reply: Reply) {
        self.reply
            .send(reply)
            .unwrap_or_else(|_| panic!("request must still be waiting"));
    }
}

struct Reply {
    bytes: Vec<u8>,
    tail: Option<oneshot::Receiver<Vec<u8>>>,
}

impl Reply {
    fn full() -> Self {
        Self::new(200, "", BODY, BODY.len())
    }

    fn suffix() -> Self {
        Self::new(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6)
    }

    fn new(status: u16, headers: &str, body: &[u8], length: usize) -> Self {
        let mut bytes = format!(
            "HTTP/1.1 {status} Response\r\nContent-Length: {length}\r\n{headers}Connection: close\r\n\r\n"
        ).into_bytes();
        bytes.extend_from_slice(body);
        Self { bytes, tail: None }
    }
}

struct Server {
    url: String,
    incoming: mpsc::UnboundedReceiver<Request>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    async fn start() -> Self {
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

    async fn request(&mut self) -> Request {
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
