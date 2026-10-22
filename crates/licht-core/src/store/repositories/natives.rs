use sqlx::{Connection, SqliteConnection};

use super::download_jobs::data_directory;
use crate::{Result, SharedCache};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeGeneration {
    pub generation: String,
    pub files: Vec<NativeOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(crate) struct NativeOutput {
    pub name: String,
    pub size: i64,
    pub modified: String,
}

pub(crate) struct NativeRepository;

impl NativeRepository {
    pub async fn get(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        version: &str,
        fingerprint: &str,
    ) -> Result<Option<NativeGeneration>> {
        game.version_json(version)?;
        let directory = data_directory(game)?;
        let mut transaction = connection.begin().await?;
        let generation: Option<String> = sqlx::query_scalar("SELECT generation FROM native_generations WHERE data_directory = ?1 AND version_id = ?2 AND fingerprint = ?3")
            .bind(directory).bind(version).bind(fingerprint).fetch_optional(&mut *transaction).await?;
        let Some(generation) = generation else {
            transaction.commit().await?;
            return Ok(None);
        };
        let files = sqlx::query_as("SELECT name, size, modified FROM native_outputs WHERE data_directory = ?1 AND version_id = ?2 AND fingerprint = ?3 ORDER BY name")
            .bind(directory).bind(version).bind(fingerprint).fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(Some(NativeGeneration { generation, files }))
    }

    pub async fn put(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        version: &str,
        fingerprint: &str,
        entry: &NativeGeneration,
    ) -> Result<()> {
        game.version_json(version)?;
        let directory = data_directory(game)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("INSERT INTO native_generations (data_directory, version_id, fingerprint, generation) VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(data_directory, version_id, fingerprint) DO UPDATE SET generation = excluded.generation")
            .bind(directory).bind(version).bind(fingerprint).bind(&entry.generation).execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM native_outputs WHERE data_directory = ?1 AND version_id = ?2 AND fingerprint = ?3")
            .bind(directory).bind(version).bind(fingerprint).execute(&mut *transaction).await?;
        for file in &entry.files {
            sqlx::query("INSERT INTO native_outputs (data_directory, version_id, fingerprint, name, size, modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
                .bind(directory).bind(version).bind(fingerprint).bind(&file.name).bind(file.size).bind(&file.modified).execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn invalidate(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        version: &str,
    ) -> Result<()> {
        game.version_json(version)?;
        let directory = data_directory(game)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM native_outputs WHERE data_directory = ?1 AND version_id = ?2")
            .bind(directory)
            .bind(version)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM native_generations WHERE data_directory = ?1 AND version_id = ?2")
            .bind(directory)
            .bind(version)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
