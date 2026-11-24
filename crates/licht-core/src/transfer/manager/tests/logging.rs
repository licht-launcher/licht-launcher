use licht_testkit::logs::LogCapture;

use super::*;

struct FailedRepair {
    job: DownloadJobSnapshot,
    failed: DownloadJobSnapshot,
    folder: String,
    logs: String,
}

/// Runs a repair whose every attempt receives corrupt bytes and captures its logs.
async fn failed_repair() -> FailedRepair {
    let capture = LogCapture::start();

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

    FailedRepair {
        job,
        failed,
        folder: entry.folder,
        logs: capture.text(),
    }
}

#[tokio::test]
async fn job_logs_carry_the_job_and_instance_context() {
    let FailedRepair {
        job, folder, logs, ..
    } = failed_repair().await;

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
        line.contains(&format!("instance={folder:?}")),
        "unexpected log line: {line}"
    );
}

#[tokio::test]
async fn a_failed_job_is_logged_with_its_error() {
    let FailedRepair {
        job, failed, logs, ..
    } = failed_repair().await;

    let line = logs
        .lines()
        .find(|line| line.contains("Download job failed"))
        .unwrap_or_else(|| panic!("the failed job was not logged: {logs}"));
    assert!(line.contains("WARN"), "unexpected log line: {line}");
    assert!(
        line.contains(&format!("job_id={}", job.id)),
        "unexpected log line: {line}"
    );
    let error = failed.error.expect("the failed job keeps its error");
    assert!(line.contains(&error), "unexpected log line: {line}");
}

#[tokio::test]
async fn a_completed_job_logs_its_start_and_completion_in_its_span() {
    let capture = LogCapture::start();
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let entry = fixture.instance("Completed").await;
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    server.request().await.reply(200, "", b"hello world", 11);
    let completed = finish(manager.wait(&job.id)).await.expect("completion");
    assert_eq!(completed.status, DownloadJobStatus::Completed);
    fixture.close().await;
    let logs = capture.text();

    for message in ["Download job started", "Download job completed"] {
        let line = logs
            .lines()
            .find(|line| line.contains(message))
            .unwrap_or_else(|| panic!("{message} was not logged: {logs}"));
        assert!(line.contains("INFO"), "unexpected log line: {line}");
        assert!(
            line.contains(&format!("job_id={}", job.id)),
            "unexpected log line: {line}"
        );
    }
    assert!(logs.contains("elapsed_seconds="), "unexpected logs: {logs}");
}

#[tokio::test]
async fn pausing_and_resuming_a_job_are_logged() {
    let capture = LogCapture::start();
    let mut server = Server::new().await;
    let fixture = Fixture::new(&server.url).await;
    let entry = fixture.instance("Paused").await;
    let manager = fixture.manager();
    let job = manager
        .start(&entry.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    let tail = server.request().await.hold();
    fixture.partial(5).await;
    manager.pause(&job.id).await.expect("pause");
    manager.resume(&job.id).await.expect("resume");
    server
        .request()
        .await
        .reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
    finish(manager.wait(&job.id)).await.expect("completion");
    drop(tail);
    fixture.close().await;
    let logs = capture.text();

    for message in ["Download job paused", "Download job resumed"] {
        let line = logs
            .lines()
            .find(|line| line.contains(message))
            .unwrap_or_else(|| panic!("{message} was not logged: {logs}"));
        assert!(
            line.contains(&format!("job_id={}", job.id)),
            "unexpected log line: {line}"
        );
    }
}
