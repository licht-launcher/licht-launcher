use super::{DownloadJobStatus, encode, load, parse_id};
use crate::{CoreError, Result, SharedCache, open_db};
use rusqlite::{TransactionBehavior, params};

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
        crate::transfer::download::remove_partial_if_idle(path).await?;
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
