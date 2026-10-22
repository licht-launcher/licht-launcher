use super::*;
use crate::transfer::jobs::model::DownloadCheckpoint;

const VERSION: &str = "1.21.1";

fn cooperative_stop_executor(
    runtime: DownloadRuntime,
    job: DownloadJobSnapshot,
    context: InstallationContext,
) -> futures_util::future::BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let directory = runtime
            .game
            .instance_dir(job.instance_folder.as_deref().expect("instance"))?;
        let stopped = directory.join("worker-stopped");
        if job.operation == DownloadJobOperation::Install {
            context.stage(DownloadPhase::Downloading).await?;
            context.control.cancelled().await;
            tokio::task::yield_now().await;
            tokio::fs::write(&stopped, b"cleanup acknowledged").await?;
            return Err(CoreError::DownloadCancelled);
        }
        if !stopped.is_file() {
            return Err(std::io::Error::other("the old worker did not finish its shutdown").into());
        }
        context.stage(DownloadPhase::Verifying).await?;
        Ok(())
    })
}

#[tokio::test]
async fn repair_waits_for_the_previous_workers_cooperative_shutdown() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Cooperative repair").await;
    let manager = fixture.manager_with(cooperative_stop_executor, reqwest::Client::new());
    let job = manager
        .start(&instance.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    finish(async {
        loop {
            if fixture
                .database()
                .get_download_job(&fixture.game, &job.id)
                .await
                .expect("job")
                .phase
                == DownloadPhase::Downloading
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let repair = finish(manager.start(&instance.folder, DownloadJobOperation::Repair))
        .await
        .expect("repair");
    let completed = finish(manager.wait(&repair.id)).await.expect("completion");
    assert_eq!(
        completed.status,
        DownloadJobStatus::Completed,
        "{:?}",
        completed.error
    );
    fixture.close().await;
}

#[tokio::test]
async fn repair_serializes_its_stop_request_with_worker_finalization() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Serialized repair").await;
    let manager = fixture.manager_with(cooperative_stop_executor, reqwest::Client::new());
    let job = manager
        .start(&instance.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    finish(async {
        loop {
            if fixture
                .database()
                .get_download_job(&fixture.game, &job.id)
                .await
                .expect("job")
                .phase
                == DownloadPhase::Downloading
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let slot = manager.active_job(&job.id).await.expect("worker");
    let finalization = slot.snapshot.lock().await;
    let restarting_manager = std::sync::Arc::clone(&manager);
    let repair = tokio::spawn(async move {
        restarting_manager
            .start(&instance.folder, DownloadJobOperation::Repair)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), slot.control.cancelled())
            .await
            .is_err(),
        "Repair must acquire the finalization lock before cancelling the worker"
    );
    drop(finalization);
    let restarted = finish(async { repair.await.expect("repair request") })
        .await
        .expect("repair");
    assert_eq!(
        finish(manager.wait(&restarted.id))
            .await
            .expect("completion")
            .status,
        DownloadJobStatus::Completed
    );
    fixture.close().await;
}

async fn checkpoint_stages(fixture: &Fixture, id: &str) {
    for stage in [DownloadCheckpoint::Game, DownloadCheckpoint::Modpack] {
        fixture
            .database()
            .record_download_checkpoint(&fixture.game, id, VERSION, stage)
            .await
            .expect("checkpoint");
    }
}

async fn assert_checkpoints_cleared(fixture: &Fixture, id: &str) {
    for stage in [DownloadCheckpoint::Game, DownloadCheckpoint::Modpack] {
        assert!(
            !fixture
                .database()
                .has_download_checkpoint(&fixture.game, id, VERSION, stage)
                .await
                .expect("checkpoint query")
        );
    }
}

#[tokio::test]
async fn explicit_repair_restarts_failed_and_interrupted_jobs_without_discarding_files() {
    for (interrupted, operation) in [
        (false, DownloadJobOperation::Install),
        (true, DownloadJobOperation::Install),
        (false, DownloadJobOperation::Repair),
        (true, DownloadJobOperation::Repair),
    ] {
        let mut server = Server::new().await;
        let mut fixture = Fixture::new(&server.url).await;
        let instance = fixture.instance("Existing installation").await;
        let directory = fixture
            .game
            .instance_dir(&instance.folder)
            .expect("directory");
        std::fs::write(directory.join("save.dat"), b"saved game").expect("save");
        std::fs::create_dir_all(fixture.game.root()).expect("cache");
        std::fs::write(fixture.part(), b"hello").expect("partial");
        let job = fixture
            .database()
            .create_pending_download_job(&fixture.game, &instance.folder, Some(VERSION), operation)
            .await
            .expect("job");
        checkpoint_stages(&fixture, &job.id).await;
        fixture
            .database()
            .update_download_job(
                &fixture.game,
                &job.id,
                crate::DownloadJobUpdate {
                    status: if interrupted {
                        DownloadJobStatus::Running
                    } else {
                        DownloadJobStatus::Failed
                    },
                    component: Some(crate::DownloadComponent::Modpack),
                    phase: DownloadPhase::Downloading,
                    progress: crate::DownloadProgress {
                        finished: 1,
                        failed: 0,
                        total: 2,
                        bytes_done: 5,
                        bytes_total: 11,
                    },
                    error: Some("previous failure".into()),
                },
            )
            .await
            .expect("stored state");
        if interrupted {
            fixture.reopen_database().await;
        }
        let manager = fixture.manager();
        let repair = manager
            .start(&instance.folder, DownloadJobOperation::Repair)
            .await
            .expect("repair");
        assert_eq!(repair.id, job.id);
        assert_eq!(repair.operation, DownloadJobOperation::Repair);
        assert_eq!(repair.status, DownloadJobStatus::Queued);
        assert_eq!(repair.component, None);
        assert_eq!(
            (
                repair.progress.finished,
                repair.progress.bytes_done,
                repair.progress.total
            ),
            (0, 0, 0)
        );
        assert!(repair.error.is_none());
        assert_checkpoints_cleared(&fixture, &job.id).await;
        let request = server.request().await;
        assert!(
            request
                .headers
                .to_ascii_lowercase()
                .contains("range: bytes=5-\r\n")
        );
        request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
        let completed = finish(manager.wait(&job.id)).await.expect("completion");
        assert_eq!(completed.status, DownloadJobStatus::Completed);
        assert_eq!(completed.operation, DownloadJobOperation::Repair);
        assert_eq!(
            std::fs::read(directory.join("save.dat")).expect("preserved save"),
            b"saved game"
        );
        assert_eq!(
            std::fs::read(fixture.game.root().join("payload.bin")).expect("download"),
            BODY
        );
        fixture.close().await;
    }
}

#[tokio::test]
async fn explicit_repair_stops_live_workers_and_preserves_their_partials() {
    for paused in [false, true] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let instance = fixture.instance("Live installation").await;
        let directory = fixture
            .game
            .instance_dir(&instance.folder)
            .expect("directory");
        std::fs::write(directory.join("save.dat"), b"saved game").expect("save");
        let manager = fixture.manager();
        let job = manager
            .start(&instance.folder, DownloadJobOperation::Install)
            .await
            .expect("start");
        let tail = server.request().await.hold();
        fixture.partial(5).await;
        if paused {
            manager.pause(&job.id).await.expect("pause");
        }
        checkpoint_stages(&fixture, &job.id).await;
        let old = manager.active_job(&job.id).await.expect("old worker");
        let repair = finish(manager.start(&instance.folder, DownloadJobOperation::Repair))
            .await
            .expect("repair");
        assert_eq!(repair.operation, DownloadJobOperation::Repair);
        assert!(
            *old.stopped.borrow(),
            "old worker must stop before replacement"
        );
        let new = manager.active_job(&repair.id).await.expect("new worker");
        assert!(!std::sync::Arc::ptr_eq(&old, &new));
        assert_checkpoints_cleared(&fixture, &job.id).await;
        let request = server.request().await;
        assert!(
            request
                .headers
                .to_ascii_lowercase()
                .contains("range: bytes=5-\r\n")
        );
        assert_eq!(
            std::fs::read(fixture.part()).expect("preserved partial"),
            b"hello"
        );
        if paused {
            let repair_tail = request.hold();
            fixture.partial(5).await;
            let cancelled = finish(manager.cancel(&repair.id))
                .await
                .expect("cancel repair");
            assert_eq!(cancelled.status, DownloadJobStatus::Cancelled);
            assert!(
                directory.is_dir(),
                "cancelling Repair must preserve the instance"
            );
            crate::load_instance(&fixture.database(), &directory)
                .await
                .expect("preserved profile");
            drop(repair_tail);
        } else {
            request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
            assert_eq!(
                finish(manager.wait(&repair.id))
                    .await
                    .expect("complete")
                    .status,
                DownloadJobStatus::Completed
            );
        }
        assert_eq!(
            std::fs::read(directory.join("save.dat")).expect("save"),
            b"saved game"
        );
        drop(tail);
        fixture.close().await;
    }
}

#[tokio::test]
async fn repair_does_not_override_persisted_instance_removal_intent() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Cancelling installation").await;
    let job = fixture
        .database()
        .create_pending_download_job(
            &fixture.game,
            &instance.folder,
            Some(VERSION),
            DownloadJobOperation::Install,
        )
        .await
        .expect("job");
    fixture
        .database()
        .update_download_job(
            &fixture.game,
            &job.id,
            crate::DownloadJobUpdate {
                status: DownloadJobStatus::Cancelling,
                component: job.component,
                phase: job.phase,
                progress: job.progress,
                error: None,
            },
        )
        .await
        .expect("removal intent");
    let manager = fixture.manager();
    assert!(matches!(
        manager
            .start(&instance.folder, DownloadJobOperation::Repair)
            .await,
        Err(CoreError::DownloadJobTransition)
    ));
    assert_eq!(
        fixture
            .database()
            .get_download_job(&fixture.game, &job.id)
            .await
            .expect("job")
            .status,
        DownloadJobStatus::Cancelling
    );
    fixture.close().await;
}

#[tokio::test]
async fn a_failed_repair_reset_rolls_back_metadata_and_preserves_resumable_partials() {
    for paused in [false, true] {
        let mut server = Server::new().await;
        let fixture = Fixture::new(&server.url).await;
        let instance = fixture.instance("Atomic repair").await;
        let manager = fixture.manager();
        let job = manager
            .start(&instance.folder, DownloadJobOperation::Install)
            .await
            .expect("start");
        let tail = server.request().await.hold();
        fixture.partial(5).await;
        if paused {
            manager.pause(&job.id).await.expect("pause");
        }
        checkpoint_stages(&fixture, &job.id).await;
        let mut connection = crate::open_db(&fixture.settings).await.expect("connection");
        sqlx::raw_sql("CREATE TRIGGER reject_repair_reset BEFORE DELETE ON download_job_checkpoints BEGIN SELECT RAISE(ABORT, 'repair reset fixture'); END;")
        .execute(&mut connection).await.expect("reject reset");
        sqlx::Connection::close(connection).await.expect("close");
        assert!(
            manager
                .start(&instance.folder, DownloadJobOperation::Repair)
                .await
                .is_err()
        );
        let unchanged = fixture
            .database()
            .get_download_job(&fixture.game, &job.id)
            .await
            .expect("job");
        assert_eq!(unchanged.operation, DownloadJobOperation::Install);
        assert_eq!(
            unchanged.status,
            if paused {
                DownloadJobStatus::Paused
            } else {
                DownloadJobStatus::Interrupted
            }
        );
        assert_eq!(unchanged.created_at, job.created_at);
        assert_eq!(std::fs::read(fixture.part()).expect("partial"), b"hello");
        for stage in [DownloadCheckpoint::Game, DownloadCheckpoint::Modpack] {
            assert!(
                fixture
                    .database()
                    .has_download_checkpoint(&fixture.game, &job.id, VERSION, stage)
                    .await
                    .expect("rolled back checkpoint")
            );
        }
        let mut connection = crate::open_db(&fixture.settings).await.expect("connection");
        sqlx::query("DROP TRIGGER reject_repair_reset")
            .execute(&mut connection)
            .await
            .expect("allow writes");
        sqlx::Connection::close(connection).await.expect("close");
        manager.resume(&job.id).await.expect("resume original job");
        let request = server.request().await;
        assert!(
            request
                .headers
                .to_ascii_lowercase()
                .contains("range: bytes=5-\r\n")
        );
        request.reply(206, "Content-Range: bytes 5-10/11\r\n", b" world", 6);
        let completed = finish(manager.wait(&job.id)).await.expect("completion");
        assert_eq!(completed.status, DownloadJobStatus::Completed);
        assert_eq!(completed.operation, DownloadJobOperation::Install);
        drop(tail);
        fixture.close().await;
    }
}
