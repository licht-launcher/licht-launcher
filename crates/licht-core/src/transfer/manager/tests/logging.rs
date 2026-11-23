use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::*;

struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl Write for TestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("the log buffer lock should work")
            .write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn job_logs_carry_the_job_and_instance_context() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer_output = Arc::clone(&output);
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || TestWriter(Arc::clone(&writer_output)))
        .with_ansi(false)
        .without_time()
        .finish();
    let _default = tracing::subscriber::set_default(subscriber);
    // While only one dispatcher is registered, tracing-core computes callsite interest
    // from the default of whichever thread hits a callsite first, so a parallel test
    // could cache the job span as disabled. A second registered dispatcher makes every
    // thread take this test's subscriber into account.
    let _registered = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());

    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let entry = fixture.instance("Logged").await;
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Repair)
        .await
        .expect("start");
    for _ in 0..3 {
        server.request().await.reply(200, "", b"wrong bytes", 11);
    }
    let failed = finish(manager.wait(&job.id))
        .await
        .expect("failed snapshot");
    assert_eq!(failed.status, DownloadJobStatus::Failed);
    fixture.close().await;

    let bytes = output
        .lock()
        .expect("the log buffer lock should work")
        .clone();
    let logs = String::from_utf8(bytes).expect("logs should contain valid UTF-8");
    let line = logs
        .lines()
        .find(|line| line.contains("download attempt failed"))
        .unwrap_or_else(|| panic!("no failed attempt was logged: {logs}"));
    assert!(
        line.contains("download_job{"),
        "unexpected log line: {line}"
    );
    assert!(
        line.contains(&format!("job_id={}", job.id)),
        "unexpected log line: {line}"
    );
    assert!(
        line.contains("operation=Repair"),
        "unexpected log line: {line}"
    );
    assert!(
        line.contains(&format!("instance={:?}", entry.folder)),
        "unexpected log line: {line}"
    );
}
