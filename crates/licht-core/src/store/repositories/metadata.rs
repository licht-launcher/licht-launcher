use sqlx::{Connection, SqliteConnection};

use crate::Result;

pub(crate) struct MetadataRepository;

impl MetadataRepository {
    pub async fn get(connection: &mut SqliteConnection, key: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT value FROM meta WHERE key = ?1")
            .bind(key)
            .fetch_optional(connection)
            .await?)
    }

    pub async fn set(connection: &mut SqliteConnection, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(connection)
        .await?;
        Ok(())
    }

    pub async fn clear(connection: &mut SqliteConnection, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM meta WHERE key = ?1")
            .bind(key)
            .execute(connection)
            .await?;
        Ok(())
    }

    pub async fn update_account(
        connection: &mut SqliteConnection,
        uuid: &str,
        remember: bool,
    ) -> Result<()> {
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let raw = Self::get(&mut transaction, "microsoft_account_uuids")
            .await?
            .unwrap_or_default();
        let mut uuids: Vec<_> = raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect();
        if remember {
            if !uuids.iter().any(|value| value == uuid) {
                uuids.push(uuid.to_string());
            }
        } else {
            uuids.retain(|value| value != uuid);
        }
        Self::set(
            &mut transaction,
            "microsoft_account_uuids",
            &uuids.join(","),
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
