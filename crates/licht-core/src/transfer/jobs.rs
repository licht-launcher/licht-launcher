use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use ts_rs::TS;

use crate::{CoreError, DownloadProgress, Result, SharedCache, open_db};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadJobOperation {
    Install,
    Repair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadJobStatus {
    Queued,
    Running,
    Paused,
    Interrupted,
    Completed,
    Cancelled,
    Failed,
}

impl DownloadJobStatus {
    fn can_transition_to(self, next: Self) -> bool {
        use DownloadJobStatus::*;
        match self {
            Completed | Cancelled => false,
            _ if self == next => true,
            Queued => matches!(next, Running | Paused | Interrupted | Cancelled | Failed),
            Running => matches!(next, Paused | Interrupted | Completed | Cancelled | Failed),
            Paused | Interrupted | Failed => matches!(next, Queued | Running | Cancelled),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadComponent {
    Minecraft,
    Java,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    Modpack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum DownloadPhase {
    Preparing,
    Downloading,
    Verifying,
    Extracting,
    Installing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobSnapshot {
    pub id: String,
    pub instance_folder: Option<String>,
    pub version_id: Option<String>,
    pub operation: DownloadJobOperation,
    pub status: DownloadJobStatus,
    pub component: Option<DownloadComponent>,
    pub phase: DownloadPhase,
    pub progress: DownloadProgress,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub java_major: Option<u32>,
}

/// Only core callers update lifecycle state; frontend commands cannot set progress.
#[derive(Debug, Clone)]
pub struct DownloadJobUpdate {
    pub status: DownloadJobStatus,
    pub component: Option<DownloadComponent>,
    pub phase: DownloadPhase,
    pub progress: DownloadProgress,
    pub error: Option<String>,
}

pub fn create_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    folder: &str,
    version_id: &str,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    create_pending_download_job(settings, game, folder, Some(version_id), operation)
}

pub fn create_pending_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    folder: &str,
    version_id: Option<&str>,
    operation: DownloadJobOperation,
) -> Result<DownloadJobSnapshot> {
    let directory = validate_context(game, folder, version_id)?;
    let mut connection = open_db(settings)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let completed = encode(&DownloadJobStatus::Completed)?;
    let cancelled = encode(&DownloadJobStatus::Cancelled)?;
    let existing: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM download_jobs
         WHERE data_directory = ?1 AND instance_folder = ?2 AND status NOT IN (?3, ?4))",
        params![directory, folder, completed, cancelled],
        |row| row.get(0),
    )?;
    if existing {
        return Err(CoreError::DownloadJobExists);
    }
    let now = timestamp()?;
    let progress = DownloadProgress {
        finished: 0,
        failed: 0,
        total: 0,
        bytes_done: 0,
        bytes_total: 0,
    };
    transaction.execute(
        "INSERT INTO download_jobs
         (data_directory, instance_folder, version_id, operation, status, component,
          phase, progress, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?8)",
        params![
            directory,
            folder,
            version_id,
            encode(&operation)?,
            encode(&DownloadJobStatus::Queued)?,
            encode(&DownloadPhase::Preparing)?,
            encode(&progress)?,
            now
        ],
    )?;
    let id = transaction.last_insert_rowid().to_string();
    transaction.commit()?;
    Ok(DownloadJobSnapshot {
        id,
        instance_folder: Some(folder.to_string()),
        version_id: version_id.map(str::to_string),
        operation,
        status: DownloadJobStatus::Queued,
        component: None,
        phase: DownloadPhase::Preparing,
        progress,
        created_at: now,
        updated_at: now,
        error: None,
        java_major: None,
    })
}

/// Lists unfinished jobs for the currently selected data directory.
pub fn list_download_jobs(
    settings: &SharedCache,
    game: &SharedCache,
) -> Result<Vec<DownloadJobSnapshot>> {
    let directory = data_directory(game)?;
    let connection = open_db(settings)?;
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM download_jobs WHERE data_directory = ?1
         AND status NOT IN (?2, ?3) ORDER BY id"
    ))?;
    let rows = statement.query_map(
        params![
            directory,
            encode(&DownloadJobStatus::Completed)?,
            encode(&DownloadJobStatus::Cancelled)?
        ],
        read_row,
    )?;
    rows.map(|row| decode_row(row?, game)).collect()
}

pub fn get_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
) -> Result<DownloadJobSnapshot> {
    let connection = open_db(settings)?;
    load(&connection, game, id)
}

pub fn update_download_job(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
    update: DownloadJobUpdate,
) -> Result<DownloadJobSnapshot> {
    validate_progress(update.progress)?;
    validate_phase(update.phase, update.component)?;
    validate_completion(update.status, update.progress)?;
    let mut connection = open_db(settings)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut job = load(&transaction, game, id)?;
    if !job.status.can_transition_to(update.status) {
        return Err(CoreError::DownloadJobTransition);
    }
    job.status = update.status;
    job.component = update.component;
    job.phase = update.phase;
    job.progress = update.progress;
    job.error = update.error;
    job.updated_at = timestamp()?.max(job.updated_at);
    transaction.execute(
        "UPDATE download_jobs SET status = ?1, component = ?2, phase = ?3,
         progress = ?4, updated_at = ?5, error = ?8 WHERE id = ?6 AND data_directory = ?7",
        params![
            encode(&job.status)?,
            job.component.as_ref().map(encode).transpose()?,
            encode(&job.phase)?,
            encode(&job.progress)?,
            job.updated_at,
            parse_id(id)?,
            data_directory(game)?,
            job.error
        ],
    )?;
    transaction.commit()?;
    Ok(job)
}

/// Called once at launcher startup. Recovery never schedules network activity.
pub fn recover_download_jobs(settings: &SharedCache, game: &SharedCache) -> Result<usize> {
    let connection = open_db(settings)?;
    let changed = connection.execute(
        "UPDATE download_jobs SET status = ?1, updated_at = MAX(updated_at, ?2)
         WHERE data_directory = ?3 AND status IN (?4, ?5)",
        params![
            encode(&DownloadJobStatus::Interrupted)?,
            timestamp()?,
            data_directory(game)?,
            encode(&DownloadJobStatus::Queued)?,
            encode(&DownloadJobStatus::Running)?
        ],
    )?;
    Ok(changed)
}

const COLUMNS: &str = "id, instance_folder, version_id, operation, status, component,
                      phase, progress, created_at, updated_at, error, java_major";

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
    java_major: Option<u32>,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredJob> {
    Ok(StoredJob {
        id: row.get(0)?,
        folder: row.get(1)?,
        version_id: row.get(2)?,
        operation: row.get(3)?,
        status: row.get(4)?,
        component: row.get(5)?,
        phase: row.get(6)?,
        progress: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        error: row.get(10)?,
        java_major: row.get(11)?,
    })
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
        java_major: row.java_major,
    };
    validate_progress(job.progress)?;
    validate_phase(job.phase, job.component)?;
    validate_completion(job.status, job.progress)?;
    Ok(job)
}

fn load(connection: &Connection, game: &SharedCache, id: &str) -> Result<DownloadJobSnapshot> {
    let stored = connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM download_jobs WHERE id = ?1 AND data_directory = ?2"),
            params![parse_id(id)?, data_directory(game)?],
            read_row,
        )
        .optional()?
        .ok_or(CoreError::DownloadJobMissing)?;
    decode_row(stored, game)
}

fn data_directory(game: &SharedCache) -> Result<&str> {
    if !game.root().is_absolute() {
        return Err(invalid("the data directory must be absolute"));
    }
    game.root()
        .to_str()
        .ok_or_else(|| invalid("the data directory must be valid UTF-8"))
}

fn validate_context<'a>(
    game: &'a SharedCache,
    folder: &str,
    version_id: Option<&str>,
) -> Result<&'a str> {
    game.instance_dir(folder)?;
    if let Some(version) = version_id {
        game.version_json(version)?;
    }
    data_directory(game)
}

fn validate_progress(progress: DownloadProgress) -> Result<()> {
    if progress
        .finished
        .checked_add(progress.failed)
        .is_none_or(|count| count > progress.total)
        || (progress.bytes_total > 0 && progress.bytes_done > progress.bytes_total)
    {
        return Err(invalid("progress exceeds its totals"));
    }
    Ok(())
}

fn validate_phase(phase: DownloadPhase, component: Option<DownloadComponent>) -> Result<()> {
    if phase != DownloadPhase::Preparing && component.is_none() {
        return Err(invalid("an active phase requires a component"));
    }
    Ok(())
}

fn validate_completion(status: DownloadJobStatus, progress: DownloadProgress) -> Result<()> {
    if status == DownloadJobStatus::Completed
        && (progress.failed != 0 || progress.finished != progress.total)
    {
        return Err(invalid(
            "a completed job must have no unfinished or failed files",
        ));
    }
    Ok(())
}

fn parse_id(id: &str) -> Result<i64> {
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

// Keep persisted enum names aligned with the serde/IPC contract.
fn encode(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(CoreError::DownloadJobJson)
}

fn decode<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(CoreError::DownloadJobJson)
}

fn invalid(detail: &'static str) -> CoreError {
    CoreError::DownloadJobInvalid { detail }
}

pub(crate) fn set_job_version(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
    version: &str,
) -> Result<()> {
    game.version_json(version)?;
    let connection = open_db(settings)?;
    load(&connection, game, id)?;
    connection.execute(
        "UPDATE download_jobs SET version_id = ?1 WHERE id = ?2 AND data_directory = ?3",
        params![version, parse_id(id)?, data_directory(game)?],
    )?;
    Ok(())
}

pub(crate) fn register_artifacts(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
    paths: &[std::path::PathBuf],
) -> Result<()> {
    let mut connection = open_db(settings)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let job = load(&transaction, game, id)?;
    let instance = job
        .instance_folder
        .as_deref()
        .map(|folder| game.instance_dir(folder))
        .transpose()?;
    for path in paths {
        validate_artifact(game, instance.as_deref(), path)?;
        let path = path.to_str().ok_or(CoreError::CachePath)?;
        transaction.execute(
            "INSERT OR IGNORE INTO download_job_artifacts (job_id, path) VALUES (?1, ?2)",
            params![parse_id(id)?, path],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub(crate) async fn cleanup_artifacts(
    settings: &SharedCache,
    game: &SharedCache,
    id: &str,
) -> Result<()> {
    let (paths, instance) = {
        let connection = open_db(settings)?;
        let job = load(&connection, game, id)?;
        let mut statement = connection.prepare(
            "SELECT artifact.path FROM download_job_artifacts artifact WHERE artifact.job_id = ?1
             AND NOT EXISTS (SELECT 1 FROM download_job_artifacts other
                JOIN download_jobs job ON job.id = other.job_id
                WHERE other.path = artifact.path AND other.job_id != ?1
                AND job.status NOT IN (?2, ?3))",
        )?;
        let paths = statement
            .query_map(
                params![
                    parse_id(id)?,
                    encode(&DownloadJobStatus::Completed)?,
                    encode(&DownloadJobStatus::Cancelled)?
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        (
            paths,
            job.instance_folder
                .as_deref()
                .map(|folder| game.instance_dir(folder))
                .transpose()?,
        )
    };
    for path in paths {
        let path = std::path::Path::new(&path);
        validate_artifact(game, instance.as_deref(), path)?;
        super::download::remove_partial_if_idle(path).await?;
    }
    Ok(())
}

fn validate_artifact(
    game: &SharedCache,
    instance: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Result<()> {
    if !path.is_absolute()
        || (!path.starts_with(game.root())
            && !instance.is_some_and(|instance| path.starts_with(instance)))
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(CoreError::CachePath);
    }
    Ok(())
}

pub(crate) fn create_java_job(
    settings: &SharedCache,
    game: &SharedCache,
    major: u32,
) -> Result<DownloadJobSnapshot> {
    if ![8, 17, 21, 25].contains(&major) {
        return Err(invalid("unsupported Java major"));
    }
    let directory = data_directory(game)?;
    let mut connection = open_db(settings)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = timestamp()?;
    let progress = DownloadProgress {
        finished: 0,
        failed: 0,
        total: 0,
        bytes_done: 0,
        bytes_total: 0,
    };
    transaction.execute(
        "INSERT INTO download_jobs (data_directory, instance_folder, version_id, operation,
         status, component, phase, progress, created_at, updated_at, java_major)
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
        params![
            directory,
            format!("java-{major}"),
            encode(&DownloadJobOperation::Install)?,
            encode(&DownloadJobStatus::Queued)?,
            encode(&DownloadComponent::Java)?,
            encode(&DownloadPhase::Preparing)?,
            encode(&progress)?,
            now,
            major
        ],
    )?;
    let id = transaction.last_insert_rowid().to_string();
    let result = load(&transaction, game, &id)?;
    transaction.commit()?;
    Ok(result)
}
