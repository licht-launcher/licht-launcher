use sqlx::SqliteConnection;
use tokio::sync::oneshot;

use super::{DatabaseClient, respond, unavailable};
use crate::store::repositories::content_fingerprints::{
    ContentFingerprint, ContentFingerprintRepository,
};
use crate::{Result, SharedCache};

pub(super) enum Request {
    List(
        String,
        String,
        oneshot::Sender<Result<Vec<ContentFingerprint>>>,
    ),
    Sync {
        directory: String,
        folder: String,
        previous: Vec<ContentFingerprint>,
        current: Vec<ContentFingerprint>,
        reply: oneshot::Sender<Result<()>>,
    },
}

impl DatabaseClient {
    pub(crate) async fn content_fingerprints(
        &self,
        game: &SharedCache,
        folder: &str,
    ) -> Result<Vec<ContentFingerprint>> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Content(Request::List(
            game.root().to_string_lossy().into_owned(),
            folder.to_string(),
            reply,
        )))
        .await?;
        response.await.map_err(|_| unavailable())?
    }

    pub(crate) async fn sync_content_fingerprints(
        &self,
        game: &SharedCache,
        folder: &str,
        previous: Vec<ContentFingerprint>,
        current: Vec<ContentFingerprint>,
    ) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.send(super::Request::Content(Request::Sync {
            directory: game.root().to_string_lossy().into_owned(),
            folder: folder.to_string(),
            previous,
            current,
            reply,
        }))
        .await?;
        response.await.map_err(|_| unavailable())?
    }
}

pub(super) async fn execute(connection: &mut SqliteConnection, request: Request) {
    match request {
        Request::List(directory, folder, reply) => respond(
            reply,
            ContentFingerprintRepository::list(connection, &directory, &folder).await,
        ),
        Request::Sync {
            directory,
            folder,
            previous,
            current,
            reply,
        } => respond(
            reply,
            ContentFingerprintRepository::sync(
                connection, &directory, &folder, &previous, &current,
            )
            .await,
        ),
    }
}
