use sqlx::{Connection, SqliteConnection};

use crate::Result;
use crate::modpacks::mrpack::storage::{StoredModpack, StoredModpackFile, StoredModpackHeader};

pub(crate) struct ModpackRepository;

impl ModpackRepository {
    pub async fn header(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<Option<StoredModpackHeader>> {
        Ok(sqlx::query_as("SELECT format_version, game, version_id, name, summary FROM modpack_indexes WHERE instances_directory = ?1 AND instance_folder = ?2")
            .bind(directory).bind(folder).fetch_optional(connection).await?)
    }

    pub async fn dependencies(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<Vec<(String, String)>> {
        Ok(sqlx::query_as("SELECT name, version FROM modpack_dependencies WHERE instances_directory = ?1 AND instance_folder = ?2 ORDER BY name")
            .bind(directory).bind(folder).fetch_all(connection).await?)
    }

    pub async fn files(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<Vec<StoredModpackFile>> {
        Ok(sqlx::query_as("SELECT relative_path, sha1, sha512, url, size FROM modpack_files WHERE instances_directory = ?1 AND instance_folder = ?2 ORDER BY position")
            .bind(directory).bind(folder).fetch_all(connection).await?)
    }

    pub async fn replace(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
        index: &StoredModpack,
    ) -> Result<()> {
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        Self::delete(&mut transaction, directory, folder).await?;
        let header = &index.header;
        sqlx::query("INSERT INTO modpack_indexes (instances_directory, instance_folder, format_version, game, version_id, name, summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(directory).bind(folder).bind(header.format_version).bind(&header.game).bind(&header.version_id).bind(&header.name).bind(&header.summary)
            .execute(&mut *transaction).await?;
        for (name, version) in &index.dependencies {
            sqlx::query("INSERT INTO modpack_dependencies (instances_directory, instance_folder, name, version) VALUES (?1, ?2, ?3, ?4)")
                .bind(directory).bind(folder).bind(name).bind(version).execute(&mut *transaction).await?;
        }
        for (position, file) in index.files.iter().enumerate() {
            sqlx::query("INSERT INTO modpack_files (instances_directory, instance_folder, position, relative_path, sha1, sha512, url, size) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
                .bind(directory).bind(folder).bind(i64::try_from(position).map_err(|_| crate::CoreError::ModpackArchive)?)
                .bind(&file.relative_path).bind(&file.sha1).bind(&file.sha512).bind(&file.url).bind(file.size)
                .execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn delete(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<()> {
        // Children are removed explicitly, including on connections without foreign_keys enabled.
        for statement in [
            "DELETE FROM modpack_files WHERE instances_directory = ?1 AND instance_folder = ?2",
            "DELETE FROM modpack_dependencies WHERE instances_directory = ?1 AND instance_folder = ?2",
            "DELETE FROM modpack_indexes WHERE instances_directory = ?1 AND instance_folder = ?2",
        ] {
            sqlx::query(statement)
                .bind(directory)
                .bind(folder)
                .execute(&mut *connection)
                .await?;
        }
        Ok(())
    }

    pub async fn duplicate(
        connection: &mut SqliteConnection,
        directory: &str,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        Self::delete(&mut transaction, directory, destination).await?;
        sqlx::query("INSERT INTO modpack_indexes SELECT instances_directory, ?3, format_version, game, version_id, name, summary FROM modpack_indexes WHERE instances_directory = ?1 AND instance_folder = ?2")
            .bind(directory).bind(source).bind(destination).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO modpack_dependencies SELECT instances_directory, ?3, name, version FROM modpack_dependencies WHERE instances_directory = ?1 AND instance_folder = ?2")
            .bind(directory).bind(source).bind(destination).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO modpack_files SELECT instances_directory, ?3, position, relative_path, sha1, sha512, url, size FROM modpack_files WHERE instances_directory = ?1 AND instance_folder = ?2")
            .bind(directory).bind(source).bind(destination).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabaseService, SharedCache};

    fn fixture() -> SharedCache {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let name = format!("licht-modpack-db-{}-{stamp}", std::process::id());
        SharedCache::at(std::env::temp_dir().join(name))
    }

    fn index() -> StoredModpack {
        StoredModpack {
            header: StoredModpackHeader {
                format_version: Some(1),
                game: Some("minecraft".into()),
                version_id: Some("v1".into()),
                name: Some("Fixture".into()),
                summary: None,
            },
            dependencies: vec![
                ("minecraft".into(), "1.20.1".into()),
                ("fabric-loader".into(), "0.16.0".into()),
            ],
            files: vec![StoredModpackFile {
                relative_path: "mods/a.jar".into(),
                sha1: "a".repeat(40),
                sha512: "b".repeat(128),
                url: "https://cdn.modrinth.com/data/a.jar".into(),
                size: 20,
            }],
        }
    }

    #[tokio::test]
    async fn persists_isolates_duplicates_and_deletes_structured_indexes() {
        let cache = fixture();
        let instances = cache.root().join("instances");
        let other = cache.root().join("other-instances");
        let service = DatabaseService::start(cache.clone())
            .await
            .expect("service");
        let client = service.client();
        let original = index();
        let settings = crate::load_settings(&cache).expect("settings");
        crate::create_instance(
            &instances,
            "First",
            "1.20.1",
            &crate::global_launch(&settings),
        )
        .expect("instance");
        client
            .replace_modpack(&instances, "First", original.clone())
            .await
            .expect("save");
        assert!(
            client
                .modpack_header(&other, "First")
                .await
                .expect("isolated directory")
                .is_none()
        );
        assert!(
            client
                .modpack_header(&instances, "Second")
                .await
                .expect("isolated instance")
                .is_none()
        );
        crate::duplicate_instance(&client, &instances, "First", "Copy")
            .await
            .expect("copy");
        crate::delete_instance(&client, &instances, "First")
            .await
            .expect("delete");
        assert!(
            client
                .modpack_header(&instances, "First")
                .await
                .expect("deleted")
                .is_none()
        );
        assert!(
            client
                .modpack_files(&instances, "First")
                .await
                .expect("deleted files")
                .is_empty()
        );
        assert!(
            client
                .modpack_dependencies(&instances, "First")
                .await
                .expect("deleted dependencies")
                .is_empty()
        );
        assert!(instances.join("Copy").join("instance.json").exists());
        assert!(!instances.join("First").exists());
        service.shutdown().await.expect("shutdown");
        let service = DatabaseService::start(cache.clone()).await.expect("reopen");
        let client = service.client();
        assert_eq!(
            client
                .modpack_header(&instances, "Copy")
                .await
                .expect("header"),
            Some(original.header)
        );
        assert_eq!(
            client
                .modpack_files(&instances, "Copy")
                .await
                .expect("files"),
            original.files
        );
        let dependencies = client
            .modpack_dependencies(&instances, "Copy")
            .await
            .expect("dependencies");
        assert_eq!(dependencies.len(), 2);
        service.shutdown().await.expect("shutdown");
        std::fs::remove_dir_all(cache.root()).expect("cleanup");
    }

    #[tokio::test]
    async fn invalid_replacement_rolls_back_the_entire_index() {
        let cache = fixture();
        let mut connection = crate::store::db::open_db(&cache).await.expect("database");
        let original = index();
        ModpackRepository::replace(&mut connection, "instances", "First", &original)
            .await
            .expect("original");
        let mut invalid = original.clone();
        invalid.header.name = Some("Should roll back".into());
        invalid.files.push(invalid.files[0].clone());
        assert!(
            ModpackRepository::replace(&mut connection, "instances", "First", &invalid)
                .await
                .is_err()
        );
        assert_eq!(
            ModpackRepository::header(&mut connection, "instances", "First")
                .await
                .expect("header"),
            Some(original.header)
        );
        assert_eq!(
            ModpackRepository::files(&mut connection, "instances", "First")
                .await
                .expect("files"),
            original.files
        );
        connection.close().await.expect("close");
        std::fs::remove_dir_all(cache.root()).expect("cleanup");
    }
}
