use sqlx::{Connection, SqliteConnection};

use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(crate) struct ContentFingerprint {
    pub relative_path: String,
    pub size: i64,
    pub modified: String,
    pub sha1: String,
}

pub(crate) struct ContentFingerprintRepository;

impl ContentFingerprintRepository {
    pub async fn list(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<Vec<ContentFingerprint>> {
        Ok(sqlx::query_as(
            "SELECT relative_path, size, modified, sha1 FROM content_fingerprints
             WHERE data_directory = ?1 AND instance_folder = ?2 ORDER BY relative_path",
        )
        .bind(directory)
        .bind(folder)
        .fetch_all(connection)
        .await?)
    }

    pub async fn sync(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
        previous: &[ContentFingerprint],
        current: &[ContentFingerprint],
    ) -> Result<()> {
        let previous: std::collections::BTreeMap<_, _> = previous
            .iter()
            .map(|entry| (entry.relative_path.as_str(), entry))
            .collect();
        let current: std::collections::BTreeMap<_, _> = current
            .iter()
            .map(|entry| (entry.relative_path.as_str(), entry))
            .collect();
        if previous == current {
            return Ok(());
        }
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        for path in previous.keys().filter(|path| !current.contains_key(*path)) {
            sqlx::query("DELETE FROM content_fingerprints WHERE data_directory = ?1 AND instance_folder = ?2 AND relative_path = ?3")
                .bind(directory).bind(folder).bind(path).execute(&mut *transaction).await?;
        }
        for (path, entry) in &current {
            if previous.get(path) == Some(entry) {
                continue;
            }
            sqlx::query(
                "INSERT INTO content_fingerprints (data_directory, instance_folder, relative_path, size, modified, sha1)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(data_directory, instance_folder, relative_path) DO UPDATE SET
                    size = excluded.size, modified = excluded.modified, sha1 = excluded.sha1",
            )
            .bind(directory).bind(folder).bind(path).bind(entry.size).bind(&entry.modified).bind(&entry.sha1)
            .execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabaseService, SharedCache};

    fn cache() -> SharedCache {
        SharedCache::at(std::env::temp_dir().join(format!(
            "licht-content-repository-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        )))
    }

    fn entry(path: &str, size: i64) -> ContentFingerprint {
        ContentFingerprint {
            relative_path: path.to_string(),
            size,
            modified: "100:0".to_string(),
            sha1: "a".repeat(40),
        }
    }

    #[tokio::test]
    async fn content_index_persists_and_isolates_instances_and_data_directories() {
        let cache = cache();
        let other = SharedCache::at(cache.root().join("other-data"));
        let service = DatabaseService::start(cache.clone())
            .await
            .expect("service");
        let client = service.client();
        let entries = vec![entry("mods/one.jar", 3)];
        client
            .sync_content_fingerprints(&cache, "First", vec![], entries.clone())
            .await
            .expect("save");
        assert!(
            client
                .content_fingerprints(&cache, "Second")
                .await
                .expect("other instance")
                .is_empty()
        );
        assert!(
            client
                .content_fingerprints(&other, "First")
                .await
                .expect("other data directory")
                .is_empty()
        );
        service.shutdown().await.expect("shutdown");

        let service = DatabaseService::start(cache.clone())
            .await
            .expect("restart");
        assert_eq!(
            service
                .client()
                .content_fingerprints(&cache, "First")
                .await
                .expect("persisted index"),
            entries
        );
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn content_index_sync_updates_adds_and_removes_only_its_instance_entries() {
        let cache = cache();
        let service = DatabaseService::start(cache.clone())
            .await
            .expect("service");
        let client = service.client();
        let initial = vec![entry("mods/one.jar", 3), entry("mods/removed.jar", 4)];
        client
            .sync_content_fingerprints(&cache, "First", vec![], initial.clone())
            .await
            .expect("initial");
        client
            .sync_content_fingerprints(&cache, "Second", vec![], initial.clone())
            .await
            .expect("other instance");
        let current = vec![entry("mods/added.jar", 6), entry("mods/one.jar", 5)];
        client
            .sync_content_fingerprints(&cache, "First", initial.clone(), current.clone())
            .await
            .expect("sync");
        assert_eq!(
            client
                .content_fingerprints(&cache, "First")
                .await
                .expect("updated"),
            current
        );
        assert_eq!(
            client
                .content_fingerprints(&cache, "Second")
                .await
                .expect("untouched"),
            initial
        );
        service.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn content_index_sync_rolls_back_removals_when_an_update_fails() {
        let cache = cache();
        let service = DatabaseService::start(cache.clone())
            .await
            .expect("service");
        let client = service.client();
        let initial = vec![entry("mods/removed.jar", 4)];
        client
            .sync_content_fingerprints(&cache, "First", vec![], initial.clone())
            .await
            .expect("initial");
        let mut invalid = entry("mods/invalid.jar", 5);
        invalid.sha1 = "invalid".to_string();
        assert!(
            client
                .sync_content_fingerprints(&cache, "First", initial.clone(), vec![invalid])
                .await
                .is_err()
        );
        assert_eq!(
            client
                .content_fingerprints(&cache, "First")
                .await
                .expect("rolled back"),
            initial
        );
        service.shutdown().await.expect("shutdown");
    }
}
