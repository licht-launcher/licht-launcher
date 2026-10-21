use super::super::*;
use super::SHA1;
use crate::{Arch, DownloadChecksum, DownloadComponent, OsName, VerifiedDownloadTask};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

fn fixture_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let mut connection = crate::open_db(&runtime.settings).await?;
        let url = crate::get_meta(&mut connection, "test_download_url")
            .await?
            .ok_or(CoreError::VersionMissing)?;
        let finish_after_cancel = crate::get_meta(&mut connection, "test_finish_after_cancel")
            .await?
            .is_some();
        sqlx::Connection::close(connection).await?;
        let component = if job.java_major.is_some() {
            DownloadComponent::Java
        } else {
            DownloadComponent::Minecraft
        };
        let context = context.component(component);
        let task = VerifiedDownloadTask {
            url,
            destination: runtime.game.root().join("payload.bin"),
            checksum: DownloadChecksum::Sha1(SHA1.into()),
            size: 11,
        };
        context
            .verified_queue(&runtime.client, &[task], context.options()?)
            .await?;
        context.checkpoint().await?;
        if let Some(folder) = &job.instance_folder {
            crate::configure_instance_version(
                &runtime.database,
                &runtime.game.instances_dir()?,
                folder,
                "1.21.1",
            )
            .await?;
            crate::complete_instance_preparation(
                &runtime.database,
                &runtime.game.instances_dir()?,
                folder,
            )
            .await?;
        }
        if let Some(major) = job.java_major {
            let root = runtime
                .game
                .root()
                .join("runtime")
                .join(format!("temurin{major}-jre{major}.0.1-linux_x64"));
            std::fs::create_dir_all(root.join("bin"))?;
            let executable = root.join("bin").join("java");
            std::fs::write(&executable, b"test executable")?;
            std::fs::write(
                root.join("release"),
                format!("JAVA_VERSION=\"{major}.0.1\""),
            )?;
            crate::set_java_path(&runtime.settings, major, &executable)?;
        }
        if finish_after_cancel {
            context.control.cancelled().await;
        }
        Ok(())
    })
}

pub(super) async fn finish<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .expect("operation timeout")
}
pub(super) async fn wait_until(mut predicate: impl FnMut() -> bool) {
    finish(async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
}

pub(super) struct Fixture {
    root: PathBuf,
    pub(super) settings: SharedCache,
    pub(super) game: SharedCache,
    database: Option<crate::DatabaseService>,
}
static FIXTURE_NUMBER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
impl Fixture {
    pub(super) async fn new(url: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = FIXTURE_NUMBER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "licht-functional-download-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        let settings = SharedCache::at(root.join("settings"));
        let game = SharedCache::at(root.join("game").join("data"));
        let mut connection = crate::open_db(&settings).await.expect("database");
        crate::set_meta(&mut connection, "test_download_url", url)
            .await
            .expect("fixture URL");
        sqlx::Connection::close(connection)
            .await
            .expect("close fixture connection");
        let database = Some(
            crate::DatabaseService::start(settings.clone())
                .await
                .expect("database service"),
        );
        Self {
            root,
            settings,
            game,
            database,
        }
    }
    pub(super) fn database(&self) -> crate::DatabaseClient {
        self.database.as_ref().expect("database owner").client()
    }
    pub(super) async fn close(mut self) {
        if let Some(database) = self.database.take() {
            database.shutdown().await.expect("close database service");
        }
    }
    pub(super) async fn instance(&self, name: &str) -> InstanceEntry {
        create_pending_instance(
            &self.database(),
            &self.game.instances_dir().expect("instances"),
            name,
            InstanceSource::Vanilla {
                game_version: "1.21.1".into(),
            },
            &global_launch(&load_settings(&self.settings).expect("settings")),
        )
        .await
        .expect("instance before network activity")
    }
    pub(super) fn manager(&self) -> Arc<DownloadManager> {
        self.manager_with(fixture_executor, reqwest::Client::new())
    }
    pub(super) fn manager_with(
        &self,
        executor: Executor,
        client: reqwest::Client,
    ) -> Arc<DownloadManager> {
        let mut manager = DownloadManager::new(
            self.settings.clone(),
            self.game.clone(),
            LaunchEnvironment {
                os: OsName::Linux,
                arch: Arch::X86_64,
                os_version: String::new(),
                features: BTreeMap::new(),
            },
            client,
            self.database(),
        );
        manager.executor = executor;
        Arc::new(manager)
    }
    pub(super) fn part(&self) -> PathBuf {
        self.game.root().join("payload.bin.part")
    }
    pub(super) async fn partial(&self, bytes: u64) {
        wait_until(|| std::fs::metadata(self.part()).is_ok_and(|file| file.len() == bytes)).await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Do not mask an assertion failure with a secondary cleanup error.
        if let Err(error) = std::fs::remove_dir_all(&self.root)
            && !std::thread::panicking()
        {
            panic!("fixture cleanup failed: {error}");
        }
    }
}

pub(super) struct Request {
    pub(super) headers: String,
    response: oneshot::Sender<Reply>,
}
#[derive(Debug)]
struct Reply {
    bytes: Vec<u8>,
    tail: Option<oneshot::Receiver<Vec<u8>>>,
}
impl Request {
    pub(super) fn reply(self, status: u16, headers: &str, body: &[u8], length: usize) {
        self.response
            .send(Reply::new(status, headers, body, length))
            .expect("response receiver");
    }
    pub(super) fn hold(self) -> oneshot::Sender<Vec<u8>> {
        let (tail, receiver) = oneshot::channel();
        let mut reply = Reply::new(200, "", b"hello", 11);
        reply.tail = Some(receiver);
        self.response.send(reply).expect("response receiver");
        tail
    }
}
impl Reply {
    fn new(status: u16, headers: &str, body: &[u8], length: usize) -> Self {
        let mut bytes = format!("HTTP/1.1 {status} Response\r\nContent-Length: {length}\r\n{headers}Connection: close\r\n\r\n").into_bytes();
        bytes.extend_from_slice(body);
        Self { bytes, tail: None }
    }
}

pub(super) struct Server {
    pub(super) url: String,
    pub(super) requests: mpsc::UnboundedReceiver<Request>,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    pub(super) async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("http://{}/file", listener.local_addr().expect("address"));
        let (sender, requests) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let sender = sender.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = socket.read(&mut buffer).await.expect("headers");
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let (response, reply) = oneshot::channel();
                    sender
                        .send(Request {
                            headers: String::from_utf8(bytes).expect("headers"),
                            response,
                        })
                        .expect("request receiver");
                    let reply = reply.await.expect("reply");
                    if socket.write_all(&reply.bytes).await.is_err() {
                        return;
                    }
                    if let Some(tail) = reply.tail
                        && let Ok(bytes) = tail.await
                    {
                        // Pausing deliberately disconnects this socket.
                        let _disconnected_client = socket.write_all(&bytes).await;
                    }
                });
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }
    pub(super) async fn request(&mut self) -> Request {
        finish(self.requests.recv()).await.expect("request")
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
