use std::future::Future;
use std::task::Poll;

use sha2::{Digest, Sha512};

use super::*;
use crate::{DownloadComponent, DownloadProgress};

fn verifying_executor(
    _runtime: DownloadRuntime,
    _job: DownloadJobSnapshot,
    context: InstallationContext,
) -> BoxFuture<'static, Result<()>> {
    Box::pin(async move {
        let context = context.component(DownloadComponent::Modpack);
        context.stage(DownloadPhase::Downloading).await?;
        context.stage(DownloadPhase::Verifying).await?;
        context.metadata(std::future::pending::<Result<()>>()).await
    })
}

#[tokio::test]
async fn modpack_late_pause_returns_the_current_verifying_snapshot() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Late pause");
    let manager = fixture.manager_with(verifying_executor, reqwest::Client::new());
    let job = manager
        .start(&instance.folder, DownloadJobOperation::Install)
        .await
        .expect("start");
    finish(async {
        loop {
            let snapshot = fixture
                .database()
                .get_download_job(&fixture.game, &job.id)
                .await
                .expect("job");
            if snapshot.phase == DownloadPhase::Verifying {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;

    let result = manager.pause(&job.id).await;
    manager.cancel(&job.id).await.expect("stop fixture");
    fixture.close().await;
    let snapshot =
        result.expect("a late pause must return the new phase rather than a transition error");
    assert_eq!(snapshot.status, DownloadJobStatus::Running);
    assert_eq!(snapshot.phase, DownloadPhase::Verifying);
}

#[tokio::test]
async fn modpack_cached_verification_yields_and_observes_cancellation() {
    let fixture = Fixture::new("http://127.0.0.1:1/unused").await;
    let instance = fixture.instance("Cached verification");
    let directory = fixture
        .game
        .instance_dir(&instance.folder)
        .expect("directory");
    let mods = directory.join("mods");
    std::fs::create_dir_all(&mods).expect("mods directory");
    let body = vec![b'x'; 2 * 1024 * 1024];
    let file = mods.join("cached.jar");
    std::fs::write(&file, &body).expect("cached mod");
    let index = serde_json::json!({ "files": [{
        "path": "mods/cached.jar", "fileSize": body.len(),
        "hashes": { "sha1": crate::hashes::sha1_hex(&body), "sha512": crate::hashes::hex_encode(Sha512::digest(&body).as_slice()) },
        "downloads": ["https://cdn.modrinth.com/data/fixture/cached.jar"]
    }] });
    std::fs::write(
        directory.join(crate::MODPACK_INDEX_FILE),
        serde_json::to_vec(&index).expect("index JSON"),
    )
    .expect("index file");
    let control = DownloadControl::new();
    let (sender, _receiver) = tokio::sync::mpsc::channel::<DownloadProgress>(8);
    let mut context: InstallationContext = sender.into();
    context.control = control.clone();
    let mut installation = Box::pin(crate::install_mrpack_files(
        &fixture.game,
        &fixture.settings,
        &directory,
        context,
    ));
    // The existing stage announcement yields once before inspecting cached files.
    std::future::poll_fn(|cx| {
        assert!(installation.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    std::future::poll_fn(|cx| {
        assert!(
            installation.as_mut().poll(cx).is_pending(),
            "cached verification must yield instead of monopolizing the installer poll"
        );
        Poll::Ready(())
    })
    .await;
    control.cancel();
    let result = finish(installation).await;
    assert!(matches!(result, Err(CoreError::DownloadCancelled)));
    assert_eq!(std::fs::read(&file).expect("cached mod preserved"), body);
    fixture.close().await;
}
