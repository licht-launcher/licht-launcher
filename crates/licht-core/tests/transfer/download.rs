use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use licht_core::{CoreError, Retry, download_file};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const BODY: &[u8] = b"hello world";
const SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

#[tokio::test]
async fn a_matching_download_lands_on_the_destination() {
    let destination = temporary_destination("success");
    let url = serve(vec![200]).await;

    download_file(&client(), &url, &destination, SHA1, retry(1))
        .await
        .expect("the download should succeed");

    assert_eq!(
        std::fs::read(&destination).expect("the file should exist"),
        BODY
    );
    assert!(!part_path(&destination).exists());
    cleanup(&destination);
}

#[tokio::test]
async fn a_wrong_sha1_leaves_neither_the_destination_nor_the_part() {
    let destination = temporary_destination("mismatch");
    let url = serve(vec![200]).await;
    let expected = "a".repeat(40);

    let error = download_file(&client(), &url, &destination, &expected, retry(1))
        .await
        .expect_err("the hash should be rejected");

    match error {
        CoreError::Sha1Mismatch { expected, actual } => {
            assert_eq!(expected, "a".repeat(40));
            assert_eq!(actual, SHA1);
        }
        other => panic!("expected a SHA1 mismatch, got {other}"),
    }
    assert!(!destination.exists());
    assert!(!part_path(&destination).exists());
    cleanup(&destination);
}

#[tokio::test]
async fn a_failed_attempt_is_retried() {
    let destination = temporary_destination("retry");
    let url = serve(vec![500, 200]).await;

    download_file(&client(), &url, &destination, SHA1, retry(2))
        .await
        .expect("the second attempt should succeed");

    assert_eq!(
        std::fs::read(&destination).expect("the file should exist"),
        BODY
    );
    assert!(!part_path(&destination).exists());
    cleanup(&destination);
}

#[tokio::test]
async fn exhausted_attempts_leave_no_destination() {
    let destination = temporary_destination("exhausted");
    let url = serve(vec![500, 500]).await;

    let error = download_file(&client(), &url, &destination, SHA1, retry(2))
        .await
        .expect_err("both attempts should fail");

    assert!(matches!(error, CoreError::Http(_)));
    assert!(!destination.exists());
    assert!(!part_path(&destination).exists());
    cleanup(&destination);
}

#[tokio::test]
async fn an_existing_destination_is_replaced() {
    let destination = temporary_destination("replace");
    std::fs::write(&destination, b"old").expect("the old file should be written");
    let url = serve(vec![200]).await;

    download_file(&client(), &url, &destination, SHA1, retry(1))
        .await
        .expect("the download should replace the file");

    assert_eq!(
        std::fs::read(&destination).expect("the file should exist"),
        BODY
    );
    assert!(!part_path(&destination).exists());
    cleanup(&destination);
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

fn temporary_destination(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("licht-download-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the temp directory should exist");
    directory.join("client.jar")
}

fn part_path(destination: &Path) -> PathBuf {
    let mut name = destination
        .file_name()
        .expect("the destination should have a file name")
        .to_os_string();
    name.push(".part");
    destination.with_file_name(name)
}

fn cleanup(destination: &Path) {
    if let Some(directory) = destination.parent() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

async fn serve(statuses: Vec<u16>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the test server should bind");
    let address = listener
        .local_addr()
        .expect("the test server should have an address");
    let statuses = Arc::new(Mutex::new(statuses));

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let statuses = Arc::clone(&statuses);
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

                let status = {
                    let mut statuses = statuses.lock().await;
                    if statuses.len() > 1 {
                        statuses.remove(0)
                    } else {
                        statuses[0]
                    }
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

    format!("http://{address}/file")
}
