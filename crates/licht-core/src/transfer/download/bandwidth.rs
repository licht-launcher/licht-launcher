use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::Instant;

use crate::{CoreError, Result};

const CAPACITY: usize = 64 * 1024;

#[derive(Debug)]
struct Budget {
    bytes_per_second: Option<u64>,
    tokens: f64,
    updated: Instant,
}

#[derive(Debug)]
pub struct BandwidthLimiter {
    budget: Mutex<Budget>,
    changed: Notify,
}

pub struct BandwidthPermit<'a> {
    limiter: &'a BandwidthLimiter,
    unused: usize,
}

impl BandwidthPermit<'_> {
    pub fn consume(&mut self, bytes: usize) {
        self.unused = self.unused.saturating_sub(bytes);
    }
}

impl Drop for BandwidthPermit<'_> {
    fn drop(&mut self) {
        if self.unused == 0 {
            return;
        }
        let mut budget = self
            .limiter
            .budget
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        budget.tokens = (budget.tokens + self.unused as f64).min(CAPACITY as f64);
        drop(budget);
        self.limiter.changed.notify_waiters();
    }
}

impl BandwidthLimiter {
    pub fn new() -> Self {
        Self {
            budget: Mutex::new(Budget {
                bytes_per_second: None,
                tokens: 0.0,
                updated: Instant::now(),
            }),
            changed: Notify::new(),
        }
    }

    pub fn set_limit(&self, bytes_per_second: Option<u64>) -> Result<()> {
        if bytes_per_second == Some(0) {
            return Err(CoreError::SettingsDownloadSpeed);
        }
        let mut budget = self
            .budget
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if budget.bytes_per_second == bytes_per_second {
            return Ok(());
        }
        budget.bytes_per_second = bytes_per_second;
        budget.tokens = 0.0;
        budget.updated = Instant::now();
        drop(budget);
        self.changed.notify_waiters();
        Ok(())
    }

    pub async fn acquire(&self, bytes: usize) -> BandwidthPermit<'_> {
        let bytes = bytes.min(CAPACITY);
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let delay = {
                let mut budget = self
                    .budget
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let Some(rate) = budget.bytes_per_second else {
                    return BandwidthPermit {
                        limiter: self,
                        unused: 0,
                    };
                };
                let now = Instant::now();
                budget.tokens = (budget.tokens
                    + now.duration_since(budget.updated).as_secs_f64() * rate as f64)
                    .min(CAPACITY as f64);
                budget.updated = now;
                if budget.tokens >= bytes as f64 {
                    budget.tokens -= bytes as f64;
                    return BandwidthPermit {
                        limiter: self,
                        unused: bytes,
                    };
                }
                Duration::from_secs_f64((bytes as f64 - budget.tokens) / rate as f64)
            };
            tokio::select! {
                _ = &mut changed => {},
                _ = tokio::time::sleep(delay) => {},
            }
        }
    }
}

impl Default for BandwidthLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn byte_limits_support_rates_below_one_mib_per_second() {
        let limiter = BandwidthLimiter::new();
        limiter.set_limit(Some(1000)).expect("one kB per second");
        let start = Instant::now();
        let mut permit = limiter.acquire(1000).await;
        permit.consume(1000);
        assert!(start.elapsed() >= Duration::from_secs(1));
        assert!(start.elapsed() < Duration::from_millis(1010));
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_transfers_share_one_global_budget() {
        let limiter = BandwidthLimiter::new();
        limiter.set_limit(Some(1024 * 1024)).expect("limit");
        let start = Instant::now();
        let transfer = || async {
            let mut permit = limiter.acquire(CAPACITY).await;
            permit.consume(CAPACITY);
        };
        tokio::join!(transfer(), transfer());
        assert!(start.elapsed() >= Duration::from_millis(125));
        assert!(start.elapsed() <= Duration::from_millis(130));
    }

    #[tokio::test(start_paused = true)]
    async fn partial_reads_refund_unused_budget() {
        let limiter = BandwidthLimiter::new();
        limiter.set_limit(Some(1024 * 1024)).expect("limit");
        let mut permit = limiter.acquire(CAPACITY).await;
        permit.consume(1024);
        drop(permit);
        let start = Instant::now();
        let mut next = limiter.acquire(CAPACITY).await;
        next.consume(CAPACITY);
        assert!(start.elapsed() < Duration::from_millis(5));
    }

    #[tokio::test(start_paused = true)]
    async fn removing_the_limit_wakes_waiting_transfers() {
        let limiter = BandwidthLimiter::new();
        limiter.set_limit(Some(1024 * 1024)).expect("limit");
        let waiting = limiter.acquire(CAPACITY);
        tokio::pin!(waiting);
        tokio::select! {
            _ = &mut waiting => panic!("budget was not available yet"),
            _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        }
        let start = Instant::now();
        limiter.set_limit(None).expect("unlimited");
        let _permit = waiting.await;
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test]
    async fn real_parallel_downloads_obey_the_shared_limit_and_preserve_checksums() {
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let body = Arc::new(vec![42_u8; 128 * 1024]);
        let checksum = crate::hashes::sha1_hex(&body);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture server");
        let url = format!("http://{}/payload", listener.local_addr().expect("address"));
        let payload = Arc::clone(&body);
        let server = tokio::spawn(async move {
            let mut replies = Vec::new();
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.expect("connection");
                let payload = Arc::clone(&payload);
                replies.push(tokio::spawn(async move {
                    let mut headers = Vec::new();
                    while !headers.ends_with(b"\r\n\r\n") {
                        headers.push(socket.read_u8().await.expect("request header"));
                    }
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        payload.len()
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("response headers");
                    socket.write_all(&payload).await.expect("response body");
                }));
            }
            for reply in replies {
                reply.await.expect("fixture reply");
            }
        });
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("licht-bandwidth-{}-{stamp}", std::process::id()));
        let tasks: Vec<_> = (0..2)
            .map(|index| crate::DownloadTask {
                url: url.clone(),
                destination: directory.join(format!("{index}.bin")),
                sha1: checksum.clone(),
                size: body.len() as u64,
            })
            .collect();
        let options = crate::DownloadOptions {
            limits: Arc::new(crate::TransferLimits::new()),
            download_concurrency: 2,
            write_concurrency: 2,
            retry: crate::Retry {
                attempts: 1,
                initial_backoff: Duration::ZERO,
            },
        };
        let client = reqwest::Client::new();
        let (events, _incoming) = tokio::sync::mpsc::channel(64);
        let started = std::time::Instant::now();
        options
            .limits
            .bandwidth
            .set_limit(Some(1024 * 1024))
            .expect("global limit");
        tokio::time::timeout(
            Duration::from_secs(10),
            crate::download_all_with_control(
                &client,
                &tasks,
                &options,
                &crate::DownloadControl::new(),
                events,
            ),
        )
        .await
        .expect("transfer timeout")
        .expect("downloads");
        assert!(started.elapsed() >= Duration::from_millis(240));
        for task in tasks {
            assert_eq!(
                std::fs::read(task.destination).expect("downloaded file"),
                *body
            );
        }
        server.await.expect("server");
        std::fs::remove_dir_all(directory).expect("cleanup");
    }
}
