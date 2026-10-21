use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteRow};

use crate::instance::persistence::validate;
use crate::{CoreError, Instance, Result};

pub(crate) struct InstanceRepository;

impl InstanceRepository {
    fn decode(row: SqliteRow) -> Result<(String, Instance)> {
        let installation: Option<String> = row.try_get("installation")?;
        let arguments: String = row.try_get("jvm_arguments")?;
        let instance = Instance {
            schema: crate::instance::SCHEMA,
            name: row.try_get("name")?,
            version_id: row.try_get("version_id")?,
            installation: installation
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(CoreError::Instance)?,
            min_memory_mb: row.try_get("min_memory_mb")?,
            max_memory_mb: row.try_get("max_memory_mb")?,
            jvm_arguments: serde_json::from_str(&arguments).map_err(CoreError::Instance)?,
            fullscreen: row.try_get("fullscreen")?,
            width: row.try_get("width")?,
            height: row.try_get("height")?,
            override_window: row.try_get("override_window")?,
            override_memory: row.try_get("override_memory")?,
            override_java: row.try_get("override_java")?,
            override_jvm_arguments: row.try_get("override_jvm_arguments")?,
            java_path: row.try_get("java_path")?,
            last_opened_at: row.try_get("last_opened_at")?,
        };
        validate(&instance)?;
        let folder: String = row.try_get("instance_folder")?;
        crate::instance::single_component(&folder)?;
        Ok((folder, instance))
    }

    pub async fn get(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<Instance> {
        let row = sqlx::query(
            "SELECT * FROM instances WHERE instances_directory = ?1 AND instance_folder = ?2",
        )
        .bind(directory)
        .bind(folder)
        .fetch_optional(connection)
        .await?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "instance profile is missing")
        })?;
        Ok(Self::decode(row)?.1)
    }

    pub async fn list(
        connection: &mut SqliteConnection,
        directory: &str,
    ) -> Result<Vec<(String, Instance)>> {
        sqlx::query(
            "SELECT * FROM instances WHERE instances_directory = ?1 ORDER BY name, instance_folder",
        )
        .bind(directory)
        .fetch_all(connection)
        .await?
        .into_iter()
        .map(Self::decode)
        .collect()
    }

    pub async fn write(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
        instance: &Instance,
    ) -> Result<()> {
        validate(instance)?;
        let installation = instance
            .installation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(CoreError::Instance)?;
        let arguments =
            serde_json::to_string(&instance.jvm_arguments).map_err(CoreError::Instance)?;
        sqlx::query("INSERT INTO instances (instances_directory, instance_folder, name, version_id, installation, min_memory_mb, max_memory_mb, jvm_arguments, fullscreen, width, height, override_window, override_memory, override_java, override_jvm_arguments, java_path, last_opened_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT (instances_directory, instance_folder) DO UPDATE SET name = excluded.name, version_id = excluded.version_id, installation = excluded.installation, min_memory_mb = excluded.min_memory_mb, max_memory_mb = excluded.max_memory_mb, jvm_arguments = excluded.jvm_arguments, fullscreen = excluded.fullscreen, width = excluded.width, height = excluded.height, override_window = excluded.override_window, override_memory = excluded.override_memory, override_java = excluded.override_java, override_jvm_arguments = excluded.override_jvm_arguments, java_path = excluded.java_path, last_opened_at = excluded.last_opened_at")
            .bind(directory).bind(folder).bind(&instance.name).bind(&instance.version_id).bind(installation)
            .bind(instance.min_memory_mb).bind(instance.max_memory_mb).bind(arguments).bind(instance.fullscreen)
            .bind(instance.width).bind(instance.height).bind(instance.override_window).bind(instance.override_memory)
            .bind(instance.override_java).bind(instance.override_jvm_arguments).bind(&instance.java_path).bind(&instance.last_opened_at)
            .execute(connection).await?;
        Ok(())
    }

    pub async fn delete(
        connection: &mut SqliteConnection,
        directory: &str,
        folder: &str,
    ) -> Result<()> {
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        super::modpacks::ModpackRepository::delete(&mut transaction, directory, folder).await?;
        sqlx::query(
            "DELETE FROM instances WHERE instances_directory = ?1 AND instance_folder = ?2",
        )
        .bind(directory)
        .bind(folder)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
