use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use licht_core::{
    CoreError, DEFAULT_CONCURRENCY, DEFAULT_RETRY, DownloadProgress, DownloadTask, Retry,
    TransferLimits, download_all, download_file,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

const BODY: &[u8] = b"hello world";
const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

#[test]
fn default_queue_limits_are_eight_workers_and_three_attempts() {
    assert_eq!(DEFAULT_CONCURRENCY, 8);
    assert_eq!(DEFAULT_RETRY.attempts, 3);
    assert_eq!(DEFAULT_RETRY.initial_backoff, Duration::from_millis(200));
}

#[tokio::test]
async fn every_file_is_reported_when_the_queue_finishes() {
    let directory = temporary_directory("finished");
    let url = serve(ServeMode::Ok).await;
    let tasks = tasks(&directory, &url, 4);
    let (progress, mut incoming) = mpsc::channel(64);
    let limits = Arc::new(TransferLimits::new());

    download_all(&client(), &tasks, limits, 2, 8, retry(1), progress)
        .await
        .expect("every download should succeed");

    let events = collect(&mut incoming).await;
    assert_eq!(
        events.last().copied(),
        Some(DownloadProgress {
            finished: 4,
            failed: 0,
            total: 4,
            bytes_done: 4 * BODY.len() as u64,
            bytes_total: 4 * BODY.len() as u64,
        })
    );
    for task in &tasks {
        assert_eq!(std::fs::read(&task.destination).expect("file"), BODY);
    }
    cleanup(&directory);
}

#[tokio::test]
async fn the_download_limit_caps_open_connections() {
    let directory = temporary_directory("limit");
    let gate = Arc::new(Gate::new());
    let url = serve(ServeMode::Hold(Arc::clone(&gate))).await;
    let tasks = tasks(&directory, &url, 4);
    let (progress, _incoming) = mpsc::channel(64);
    let client = client();
    let limits = Arc::new(TransferLimits::new());

    let queue = tokio::spawn(async move {
        download_all(&client, &tasks, limits, 2, 8, retry(1), progress).await
    });

    while gate.active.load(Ordering::SeqCst) < 2 {
        tokio::task::yield_now().await;
    }
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(gate.max_active.load(Ordering::SeqCst), 2);
    assert_eq!(gate.active.load(Ordering::SeqCst), 2);

    gate.open
        .send(true)
        .expect("the gate should still have waiters");
    queue
        .await
        .expect("the queue task should finish")
        .expect("every download should succeed");
    cleanup(&directory);
}

#[tokio::test]
async fn two_queues_share_the_global_download_limit() {
    let directory = temporary_directory("shared");
    let gate = Arc::new(Gate::new());
    let url = serve(ServeMode::Hold(Arc::clone(&gate))).await;
    let limits = Arc::new(TransferLimits::new());
    let left = tasks(&directory.join("left"), &url, 3);
    let right = tasks(&directory.join("right"), &url, 3);
    std::fs::create_dir_all(directory.join("left")).expect("left");
    std::fs::create_dir_all(directory.join("right")).expect("right");
    let client = client();
    let (progress_a, _) = mpsc::channel(64);
    let (progress_b, _) = mpsc::channel(64);

    let first = {
        let client = client.clone();
        let limits = Arc::clone(&limits);
        tokio::spawn(async move {
            download_all(&client, &left, limits, 2, 8, retry(1), progress_a).await
        })
    };
    let second = {
        let client = client.clone();
        let limits = Arc::clone(&limits);
        tokio::spawn(async move {
            download_all(&client, &right, limits, 2, 8, retry(1), progress_b).await
        })
    };

    while gate.active.load(Ordering::SeqCst) < 2 {
        tokio::task::yield_now().await;
    }
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(gate.max_active.load(Ordering::SeqCst), 2);

    gate.open
        .send(true)
        .expect("the gate should still have waiters");
    first
        .await
        .expect("join")
        .expect("left queue should succeed");
    second
        .await
        .expect("join")
        .expect("right queue should succeed");
    cleanup(&directory);
}

#[tokio::test]
async fn zero_write_concurrency_is_rejected() {
    let directory = temporary_directory("write-zero");
    let url = serve(ServeMode::Ok).await;
    let tasks = tasks(&directory, &url, 1);
    let (progress, _) = mpsc::channel(8);
    let error = download_all(
        &client(),
        &tasks,
        Arc::new(TransferLimits::new()),
        2,
        0,
        retry(1),
        progress,
    )
    .await
    .expect_err("write concurrency zero");
    assert!(matches!(error, CoreError::WriteConcurrency));
    cleanup(&directory);
}

#[tokio::test]
async fn skips_when_the_destination_already_matches_sha1() {
    let directory = temporary_directory("skip");
    let destination = directory.join("cached.jar");
    std::fs::write(&destination, BODY).expect("seed");
    let gate = Arc::new(Gate::new());
    let url = serve(ServeMode::Hold(Arc::clone(&gate))).await;

    download_file(&client(), &url, &destination, SHA1, retry(1))
        .await
        .expect("existing file should skip the download");

    assert_eq!(gate.active.load(Ordering::SeqCst), 0);
    assert_eq!(gate.max_active.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(&destination).expect("file"), BODY);
    cleanup(&directory);
}

#[tokio::test]
async fn two_parallel_downloads_to_the_same_path_do_not_corrupt() {
    let directory = temporary_directory("race");
    let destination = directory.join("shared.jar");
    let url = serve(ServeMode::Ok).await;
    let client = client();
    let first = {
        let client = client.clone();
        let url = url.clone();
        let destination = destination.clone();
        tokio::spawn(
            async move { download_file(&client, &url, &destination, SHA1, retry(1)).await },
        )
    };
    let second = {
        let client = client.clone();
        let url = url.clone();
        let destination = destination.clone();
        tokio::spawn(
            async move { download_file(&client, &url, &destination, SHA1, retry(1)).await },
        )
    };

    first.await.expect("join").expect("first download");
    second.await.expect("join").expect("second download");
    assert_eq!(std::fs::read(&destination).expect("file"), BODY);
    assert!(!directory.join("shared.jar.part").exists());
    cleanup(&directory);
}

#[tokio::test]
async fn one_failure_is_reported_after_the_other_files_land() {
    let directory = temporary_directory("partial");
    let url = serve(ServeMode::FailPath).await;
    let mut tasks = tasks(&directory, &format!("{url}/ok"), 2);
    tasks.push(DownloadTask {
        url: format!("{url}/fail"),
        destination: directory.join("failed.jar"),
        sha1: SHA1.to_string(),
        size: BODY.len() as u64,
    });
    let (progress, mut incoming) = mpsc::channel(64);

    let error = download_all(
        &client(),
        &tasks,
        Arc::new(TransferLimits::new()),
        2,
        8,
        retry(1),
        progress,
    )
    .await
    .expect_err("the failing file should fail the queue");

    assert!(matches!(error, CoreError::Http(_)));
    let events = collect(&mut incoming).await;
    assert_eq!(
        events.last().copied(),
        Some(DownloadProgress {
            finished: 2,
            failed: 1,
            total: 3,
            bytes_done: 2 * BODY.len() as u64,
            bytes_total: 3 * BODY.len() as u64,
        })
    );
    assert_eq!(std::fs::read(directory.join("0.jar")).expect("file"), BODY);
    assert_eq!(std::fs::read(directory.join("1.jar")).expect("file"), BODY);
    assert!(!directory.join("failed.jar").exists());
    cleanup(&directory);
}

struct Gate {
    open: watch::Sender<bool>,
    active: AtomicUsize,
    max_active: AtomicUsize,
}

impl Gate {
    fn new() -> Self {
        let (open, _) = watch::channel(false);
        Self {
            open,
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
        }
    }
}

enum ServeMode {
    Ok,
    Hold(Arc<Gate>),
    FailPath,
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn retry(attempts: u32) -> Retry {
    Retry {
        attempts,
        initial_backoff: Duration::from_millis(1),
    }
}

fn tasks(directory: &Path, url: &str, count: usize) -> Vec<DownloadTask> {
    (0..count)
        .map(|index| DownloadTask {
            url: url.to_string(),
            destination: directory.join(format!("{index}.jar")),
            sha1: SHA1.to_string(),
            size: BODY.len() as u64,
        })
        .collect()
}

fn temporary_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("licht-queue-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the temp directory should exist");
    directory
}

fn cleanup(directory: &Path) {
    let _ = std::fs::remove_dir_all(directory);
}

async fn collect(incoming: &mut mpsc::Receiver<DownloadProgress>) -> Vec<DownloadProgress> {
    let mut events = Vec::new();
    while let Some(event) = incoming.recv().await {
        events.push(event);
    }
    events
}

async fn serve(mode: ServeMode) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the test server should bind");
    let address = listener
        .local_addr()
        .expect("the test server should have an address");
    let mode = Arc::new(mode);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mode = Arc::clone(&mode);
            tokio::spawn(async move {
                let mut incoming = Vec::new();
                let mut buffer = [0_u8; 256];
                loop {
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    if read == 0 {
                        return;
                    }
                    incoming.extend_from_slice(&buffer[..read]);
                    if incoming.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }

                if let ServeMode::Hold(gate) = mode.as_ref() {
                    let mut open = gate.open.subscribe();
                    let now = gate.active.fetch_add(1, Ordering::SeqCst) + 1;
                    gate.max_active.fetch_max(now, Ordering::SeqCst);
                    while !*open.borrow() {
                        if open.changed().await.is_err() {
                            break;
                        }
                    }
                    gate.active.fetch_sub(1, Ordering::SeqCst);
                }

                let status = match mode.as_ref() {
                    ServeMode::FailPath if request_targets_fail(&incoming) => 500,
                    _ => 200,
                };
                let reason = if status == 200 { "OK" } else { "Error" };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    BODY.len()
                );
                if socket.write_all(header.as_bytes()).await.is_err() {
                    return;
                }
                let _ = socket.write_all(BODY).await;
            });
        }
    });

    format!("http://{address}")
}

fn request_targets_fail(request: &[u8]) -> bool {
    request
        .windows(b"GET /fail ".len())
        .any(|window| window == b"GET /fail ")
}
