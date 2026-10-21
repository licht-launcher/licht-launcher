use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Connection, SqliteConnection};

use crate::transfer::jobs::model::{
    validate_cancellation_target, validate_completion, validate_phase, validate_progress,
};
use crate::{
    CoreError, DownloadComponent, DownloadJobOperation, DownloadJobSnapshot, DownloadJobStatus,
    DownloadJobUpdate, DownloadPhase, DownloadProgress, Result, SharedCache,
};

pub(crate) struct DownloadJobRepository;

impl DownloadJobRepository {
    pub async fn create(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        folder: &str,
        version: Option<&str>,
        operation: DownloadJobOperation,
    ) -> Result<DownloadJobSnapshot> {
        validate_context(game, folder, version)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM download_jobs
             WHERE data_directory = ?1 AND instance_folder = ?2 AND status NOT IN (?3, ?4))",
        )
        .bind(data_directory(game)?)
        .bind(folder)
        .bind(encode(&DownloadJobStatus::Completed)?)
        .bind(encode(&DownloadJobStatus::Cancelled)?)
        .fetch_one(&mut *transaction)
        .await?;
        if exists {
            return Err(CoreError::DownloadJobExists);
        }
        let now = timestamp()?;
        let id = sqlx::query(
            "INSERT INTO download_jobs
             (data_directory, instance_folder, version_id, operation, status, component,
              phase, progress, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?8)",
        )
        .bind(data_directory(game)?)
        .bind(folder)
        .bind(version)
        .bind(encode(&operation)?)
        .bind(encode(&DownloadJobStatus::Queued)?)
        .bind(encode(&DownloadPhase::Preparing)?)
        .bind(encode(&empty_progress())?)
        .bind(now)
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid()
        .to_string();
        let job = Self::get(&mut transaction, game, &id).await?;
        transaction.commit().await?;
        Ok(job)
    }

    pub async fn list(
        connection: &mut SqliteConnection,
        game: &SharedCache,
    ) -> Result<Vec<DownloadJobSnapshot>> {
        let rows = sqlx::query_as::<_, StoredJob>(
            "SELECT id, instance_folder AS folder, version_id, operation, status, component,
                    phase, progress, created_at, updated_at, error, java_major
             FROM download_jobs WHERE data_directory = ?1 AND status NOT IN (?2, ?3) ORDER BY id",
        )
        .bind(data_directory(game)?)
        .bind(encode(&DownloadJobStatus::Completed)?)
        .bind(encode(&DownloadJobStatus::Cancelled)?)
        .fetch_all(connection)
        .await?;
        rows.into_iter().map(|row| decode_row(row, game)).collect()
    }

    pub async fn get(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
    ) -> Result<DownloadJobSnapshot> {
        let row = sqlx::query_as::<_, StoredJob>(
            "SELECT id, instance_folder AS folder, version_id, operation, status, component,
                    phase, progress, created_at, updated_at, error, java_major
             FROM download_jobs WHERE id = ?1 AND data_directory = ?2",
        )
        .bind(parse_id(id)?)
        .bind(data_directory(game)?)
        .fetch_optional(connection)
        .await?
        .ok_or(CoreError::DownloadJobMissing)?;
        decode_row(row, game)
    }

    pub async fn update(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
        update: DownloadJobUpdate,
    ) -> Result<DownloadJobSnapshot> {
        validate_progress(update.progress)?;
        validate_phase(update.phase, update.component)?;
        validate_completion(update.status, update.progress)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let mut job = Self::get(&mut transaction, game, id).await?;
        validate_cancellation_target(&job, update.status)?;
        if !job.status.can_transition_to(update.status) {
            return Err(CoreError::DownloadJobTransition);
        }
        job.status = update.status;
        job.component = update.component;
        job.phase = update.phase;
        job.progress = update.progress;
        job.error = update.error;
        job.updated_at = timestamp()?.max(job.updated_at);
        sqlx::query(
            "UPDATE download_jobs SET status = ?1, component = ?2, phase = ?3,
             progress = ?4, updated_at = ?5, error = ?8 WHERE id = ?6 AND data_directory = ?7",
        )
        .bind(encode(&job.status)?)
        .bind(job.component.as_ref().map(encode).transpose()?)
        .bind(encode(&job.phase)?)
        .bind(encode(&job.progress)?)
        .bind(job.updated_at)
        .bind(parse_id(id)?)
        .bind(data_directory(game)?)
        .bind(&job.error)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(job)
    }

    pub async fn recover(connection: &mut SqliteConnection, game: &SharedCache) -> Result<usize> {
        let changed = sqlx::query(
            "UPDATE download_jobs SET status = ?1, updated_at = MAX(updated_at, ?2)
             WHERE data_directory = ?3 AND status IN (?4, ?5)",
        )
        .bind(encode(&DownloadJobStatus::Interrupted)?)
        .bind(timestamp()?)
        .bind(data_directory(game)?)
        .bind(encode(&DownloadJobStatus::Queued)?)
        .bind(encode(&DownloadJobStatus::Running)?)
        .execute(connection)
        .await?
        .rows_affected();
        usize::try_from(changed).map_err(|error| std::io::Error::other(error).into())
    }

    pub async fn set_version(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        id: &str,
        version: &str,
    ) -> Result<()> {
        game.version_json(version)?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        Self::get(&mut transaction, game, id).await?;
        sqlx::query(
            "UPDATE download_jobs SET version_id = ?1 WHERE id = ?2 AND data_directory = ?3",
        )
        .bind(version)
        .bind(parse_id(id)?)
        .bind(data_directory(game)?)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("DELETE FROM download_job_checkpoints WHERE job_id = ?1 AND version_id != ?2")
            .bind(parse_id(id)?)
            .bind(version)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn create_java(
        connection: &mut SqliteConnection,
        game: &SharedCache,
        major: u32,
    ) -> Result<DownloadJobSnapshot> {
        if ![8, 17, 21, 25].contains(&major) {
            return Err(invalid("unsupported Java major"));
        }
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let now = timestamp()?;
        let id = sqlx::query(
            "INSERT INTO download_jobs
             (data_directory, instance_folder, version_id, operation, status, component,
              phase, progress, created_at, updated_at, java_major)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
        )
        .bind(data_directory(game)?)
        .bind(format!("java-{major}"))
        .bind(encode(&DownloadJobOperation::Install)?)
        .bind(encode(&DownloadJobStatus::Queued)?)
        .bind(encode(&DownloadComponent::Java)?)
        .bind(encode(&DownloadPhase::Preparing)?)
        .bind(encode(&empty_progress())?)
        .bind(now)
        .bind(i64::from(major))
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid()
        .to_string();
        let job = Self::get(&mut transaction, game, &id).await?;
        transaction.commit().await?;
        Ok(job)
    }
}

#[derive(sqlx::FromRow)]
struct StoredJob {
    id: i64,
    folder: Option<String>,
    version_id: Option<String>,
    operation: String,
    status: String,
    component: Option<String>,
    phase: String,
    progress: String,
    created_at: i64,
    updated_at: i64,
    error: Option<String>,
    java_major: Option<i64>,
}

fn decode_row(row: StoredJob, game: &SharedCache) -> Result<DownloadJobSnapshot> {
    if let Some(folder) = &row.folder {
        validate_context(game, folder, row.version_id.as_deref())?;
        if row.java_major.is_some() {
            return Err(invalid("a Java task cannot target an instance"));
        }
    } else if !row
        .java_major
        .is_some_and(|major| [8, 17, 21, 25].contains(&major))
    {
        return Err(invalid("task target is missing or invalid"));
    }
    if row.id <= 0 || row.created_at < 0 || row.updated_at < row.created_at {
        return Err(invalid("invalid identity or timestamps"));
    }
    let job = DownloadJobSnapshot {
        id: row.id.to_string(),
        instance_folder: row.folder,
        version_id: row.version_id,
        operation: decode(&row.operation)?,
        status: decode(&row.status)?,
        component: row.component.as_deref().map(decode).transpose()?,
        phase: decode(&row.phase)?,
        progress: decode(&row.progress)?,
        created_at: row.created_at,
        updated_at: row.updated_at,
        error: row.error,
        java_major: row
            .java_major
            .map(|major| u32::try_from(major).map_err(|_| invalid("unsupported Java major")))
            .transpose()?,
    };
    validate_progress(job.progress)?;
    validate_cancellation_target(&job, job.status)?;
    validate_phase(job.phase, job.component)?;
    validate_completion(job.status, job.progress)?;
    Ok(job)
}

pub(crate) fn data_directory(game: &SharedCache) -> Result<&str> {
    if !game.root().is_absolute() {
        return Err(invalid("the data directory must be absolute"));
    }
    game.root()
        .to_str()
        .ok_or_else(|| invalid("the data directory must be valid UTF-8"))
}

fn validate_context(game: &SharedCache, folder: &str, version: Option<&str>) -> Result<()> {
    game.instance_dir(folder)?;
    if let Some(version) = version {
        game.version_json(version)?;
    }
    data_directory(game)?;
    Ok(())
}

pub(crate) fn parse_id(id: &str) -> Result<i64> {
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("invalid job ID"));
    }
    id.parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| invalid("invalid job ID"))
}

fn timestamp() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(std::io::Error::other)?;
    i64::try_from(elapsed.as_millis()).map_err(|error| std::io::Error::other(error).into())
}

pub(crate) fn encode(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(CoreError::DownloadJobJson)
}

fn decode<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(CoreError::DownloadJobJson)
}

fn empty_progress() -> DownloadProgress {
    DownloadProgress {
        finished: 0,
        failed: 0,
        total: 0,
        bytes_done: 0,
        bytes_total: 0,
    }
}

fn invalid(detail: &'static str) -> CoreError {
    CoreError::DownloadJobInvalid { detail }
}
