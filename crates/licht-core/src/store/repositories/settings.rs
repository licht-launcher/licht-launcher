use sqlx::{Row, SqliteConnection};

use crate::{CoreError, Result, Settings};

/// The settings kept in the database. Startup preferences stay in `settings.json`.
pub(crate) struct SettingsRepository;

impl SettingsRepository {
    pub async fn exists(connection: &mut SqliteConnection) -> Result<bool> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM settings WHERE id = 1)")
                .fetch_one(connection)
                .await?,
        )
    }

    /// Replaces the database-backed fields of `settings`. Returns `false` when no row is stored yet.
    pub async fn load_into(
        connection: &mut SqliteConnection,
        settings: &mut Settings,
    ) -> Result<bool> {
        let Some(row) = sqlx::query("SELECT * FROM settings WHERE id = 1")
            .fetch_optional(connection)
            .await?
        else {
            return Ok(false);
        };
        let arguments: String = row.try_get("jvm_arguments")?;
        let speed_limit: Option<i64> = row.try_get("download_speed_limit_bytes_per_second")?;
        settings.username = row.try_get("username")?;
        settings.fullscreen = row.try_get("fullscreen")?;
        settings.width = row.try_get("width")?;
        settings.height = row.try_get("height")?;
        settings.max_memory_mb = row.try_get("max_memory_mb")?;
        settings.jvm_arguments = serde_json::from_str(&arguments).map_err(CoreError::Settings)?;
        settings.java25 = row.try_get("java25")?;
        settings.java21 = row.try_get("java21")?;
        settings.java17 = row.try_get("java17")?;
        settings.java8 = row.try_get("java8")?;
        settings.download_concurrency = row.try_get("download_concurrency")?;
        settings.write_concurrency = row.try_get("write_concurrency")?;
        settings.download_speed_limit_bytes_per_second = speed_limit
            .map(u64::try_from)
            .transpose()
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        settings.disable_custom_title_bar = row.try_get("disable_custom_title_bar")?;
        Ok(true)
    }

    pub async fn save(connection: &mut SqliteConnection, settings: &Settings) -> Result<()> {
        let arguments =
            serde_json::to_string(&settings.jvm_arguments).map_err(CoreError::Settings)?;
        let speed_limit = settings
            .download_speed_limit_bytes_per_second
            .map(i64::try_from)
            .transpose()
            .map_err(|_| CoreError::SettingsDownloadSpeed)?;
        sqlx::query("INSERT INTO settings (id, username, fullscreen, width, height, max_memory_mb, jvm_arguments, java25, java21, java17, java8, download_concurrency, write_concurrency, download_speed_limit_bytes_per_second, disable_custom_title_bar)
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT (id) DO UPDATE SET username = excluded.username, fullscreen = excluded.fullscreen, width = excluded.width, height = excluded.height, max_memory_mb = excluded.max_memory_mb, jvm_arguments = excluded.jvm_arguments, java25 = excluded.java25, java21 = excluded.java21, java17 = excluded.java17, java8 = excluded.java8, download_concurrency = excluded.download_concurrency, write_concurrency = excluded.write_concurrency, download_speed_limit_bytes_per_second = excluded.download_speed_limit_bytes_per_second, disable_custom_title_bar = excluded.disable_custom_title_bar")
            .bind(&settings.username).bind(settings.fullscreen).bind(settings.width).bind(settings.height)
            .bind(settings.max_memory_mb).bind(arguments).bind(&settings.java25).bind(&settings.java21)
            .bind(&settings.java17).bind(&settings.java8).bind(settings.download_concurrency)
            .bind(settings.write_concurrency).bind(speed_limit).bind(settings.disable_custom_title_bar)
            .execute(connection).await?;
        Ok(())
    }
}
