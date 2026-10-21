use std::fs;
use std::path::{Component, Path, PathBuf};

use sqlx::Connection;

use super::files;
use super::progress::StartupReporter;
use crate::store::repositories::metadata::MetadataRepository;
use crate::{CoreError, Result, SharedCache};

pub(super) async fn migrate(
    home: &SharedCache,
    game: &SharedCache,
    reporter: &StartupReporter,
) -> Result<()> {
    let key = format!("installation-storage-v1:{}", game.root().display());
    let mut connection = super::db::open_db(home).await?;
    let result = async {
        if MetadataRepository::get(&mut connection, &key).await?.as_deref() == Some("1") {
            return Ok(());
        }
        migrate_icons(&mut connection, &game.instances_dir()?, reporter).await?;
        let legacy = game.root().join("modpacks");
        let artifacts: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT artifact.job_id, artifact.path, job.instance_folder FROM download_job_artifacts artifact
             JOIN download_jobs job ON job.id = artifact.job_id WHERE job.data_directory = ?1
             AND job.status NOT IN (?2, ?3)",
        ).bind(game.root().to_str().ok_or(CoreError::CachePath)?)
            .bind("\"completed\"").bind("\"cancelled\"").fetch_all(&mut connection).await?;
        let mut updates = Vec::new();
        for (id, path, folder) in artifacts {
            let source = PathBuf::from(path);
            if !source.starts_with(&legacy) { continue; }
            if source.components().any(|part| matches!(part, Component::ParentDir))
                || source.file_name().is_none_or(|name| name != "pack.zip")
            { return Err(CoreError::CachePath); }
            let folder = folder.ok_or(CoreError::CachePath)?;
            let destination = crate::modpacks::workspace::archive_path(&game.instance_dir(&folder)?)?;
            files::copy_tree_with_progress(&source, &destination, reporter)?;
            files::copy_tree_with_progress(&source.with_file_name("pack.zip.part"), &destination.with_file_name("pack.zip.part"), reporter)?;
            updates.push((id, source, destination));
        }
        let mut transaction = connection.begin().await?;
        for (id, source, destination) in updates {
            sqlx::query("INSERT OR IGNORE INTO download_job_artifacts (job_id, path) VALUES (?1, ?2)")
                .bind(id).bind(destination.to_str().ok_or(CoreError::CachePath)?).execute(&mut *transaction).await?;
            sqlx::query("DELETE FROM download_job_artifacts WHERE job_id = ?1 AND path = ?2")
                .bind(id).bind(source.to_str().ok_or(CoreError::CachePath)?).execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        remove_legacy_archives(&legacy)?;
        MetadataRepository::set(&mut connection, &key, "1").await?;
        Ok(())
    }.await;
    super::db::complete_operation(connection, result).await
}

async fn migrate_icons(
    connection: &mut sqlx::SqliteConnection,
    instances: &Path,
    reporter: &StartupReporter,
) -> Result<()> {
    if !instances.exists() {
        return Ok(());
    }
    let profiles = crate::store::repositories::instances::InstanceRepository::list(
        connection,
        instances.to_str().ok_or(CoreError::CachePath)?,
    )
    .await?;
    for (folder, profile) in profiles {
        reporter.checkpoint()?;
        let dir = instances.join(folder);
        let source = dir.join(crate::ICON_FILE);
        if !source.is_file() {
            continue;
        }
        let url = crate::instance::icon::source_icon_url(&profile);
        let destination =
            crate::instance::icon::cached_icon_path(&dir, url).ok_or(CoreError::CachePath)?;
        if !destination.is_file() {
            files::copy_file_with_progress(&source, &destination, reporter)?;
        }
        fs::remove_file(source)?;
    }
    Ok(())
}

fn remove_legacy_archives(directory: &Path) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(directory)?.file_type().is_symlink() {
        return Err(CoreError::CachePath);
    }
    for item in fs::read_dir(directory)? {
        let item = item?;
        let kind = item.file_type()?;
        if kind.is_dir() {
            remove_legacy_archives(&item.path())?;
        } else if kind.is_file()
            && (item.file_name() == "pack.zip" || item.file_name() == "pack.zip.part")
        {
            fs::remove_file(item.path())?;
        }
    }
    match fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {
            tracing::warn!("Preserving unknown files in a legacy modpack cache");
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::layout::{initialize_at, tests::fixture};

    #[tokio::test]
    async fn pending_packs_get_independent_workspaces_and_shared_cached_icons() {
        let (legacy, home) = fixture("installation-assets");
        let game = SharedCache::at(home.root().join("data"));
        let instances = game.instances_dir().expect("instances");
        let settings = crate::load_settings(&home).expect("settings");
        crate::save_settings(&home, &settings).expect("save settings");
        let source = crate::InstanceSource::Modpack {
            source: crate::ModpackSource::Modrinth,
            project_id: "fixture".to_string(),
            version_id: "v1".to_string(),
            icon_url: Some("https://cdn.modrinth.com/data/fixture/icon.png".to_string()),
        };
        let service = crate::DatabaseService::start(home.clone())
            .await
            .expect("service");
        let first = crate::create_pending_instance(
            &service.client(),
            &instances,
            "First",
            source.clone(),
            &crate::global_launch(&settings),
        )
        .await
        .expect("first instance");
        let second = crate::create_pending_instance(
            &service.client(),
            &instances,
            "Second",
            source,
            &crate::global_launch(&settings),
        )
        .await
        .expect("second instance");
        for folder in [&first.folder, &second.folder] {
            fs::write(
                instances.join(folder).join(crate::ICON_FILE),
                b"legacy icon",
            )
            .expect("icon");
        }
        let archive = game
            .root()
            .join("modpacks")
            .join("modrinth")
            .join("fixture")
            .join("v1")
            .join("pack.zip");
        fs::create_dir_all(archive.parent().expect("archive parent")).expect("directory");
        fs::write(&archive, b"archive").expect("archive");
        fs::write(archive.with_file_name("pack.zip.part"), b"partial").expect("partial");
        let mut jobs = Vec::new();
        for folder in [&first.folder, &second.folder] {
            let job = service
                .client()
                .create_pending_download_job(
                    &game,
                    folder,
                    None,
                    crate::DownloadJobOperation::Install,
                )
                .await
                .expect("job");
            service
                .client()
                .register_download_artifacts(&game, &job.id, std::slice::from_ref(&archive))
                .await
                .expect("artifact");
            jobs.push(job);
        }
        service.shutdown().await.expect("close old service");

        initialize_at(&legacy, &home)
            .await
            .expect("migrate installation assets");
        assert!(!game.root().join("modpacks").exists());
        let service = crate::DatabaseService::start(home.clone())
            .await
            .expect("reopen");
        let first_entry = crate::instance_entry(
            &service.client(),
            &instances.join(&first.folder),
            &first.folder,
        )
        .await
        .expect("first icon");
        let second_entry = crate::instance_entry(
            &service.client(),
            &instances.join(&second.folder),
            &second.folder,
        )
        .await
        .expect("second icon");
        assert_eq!(first_entry.icon_path, second_entry.icon_path);
        service.shutdown().await.expect("close service");
        assert_eq!(
            fs::read(first_entry.icon_path.expect("cached icon")).expect("icon bytes"),
            b"legacy icon"
        );
        for folder in [&first.folder, &second.folder] {
            let directory = instances.join(folder);
            assert!(!directory.join(crate::ICON_FILE).exists());
            assert_eq!(
                fs::read(directory.join(".install").join("pack.zip")).expect("archive"),
                b"archive"
            );
            assert_eq!(
                fs::read(directory.join(".install").join("pack.zip.part")).expect("partial"),
                b"partial"
            );
        }
        initialize_at(&legacy, &home)
            .await
            .expect("repeat initialization");
        let mut connection = super::super::db::open_db(&home).await.expect("database");
        for job in &jobs {
            let path: String =
                sqlx::query_scalar("SELECT path FROM download_job_artifacts WHERE job_id = ?1")
                    .bind(&job.id)
                    .fetch_one(&mut connection)
                    .await
                    .expect("artifact reference");
            assert_eq!(
                PathBuf::from(path),
                instances
                    .join(job.instance_folder.as_ref().expect("folder"))
                    .join(".install")
                    .join("pack.zip")
            );
        }
        connection.close().await.expect("close database");
        let service = crate::DatabaseService::start(home.clone())
            .await
            .expect("new service");
        let manager = crate::DownloadManager::new(
            home.clone(),
            game.clone(),
            crate::LaunchEnvironment {
                os: crate::OsName::Linux,
                arch: crate::Arch::X86_64,
                os_version: String::new(),
                features: Default::default(),
            },
            reqwest::Client::new(),
            service.client(),
        );
        manager
            .cancel(&jobs[0].id)
            .await
            .expect("cancel first installation");
        assert!(!instances.join(&first.folder).exists());
        assert_eq!(
            fs::read(
                instances
                    .join(&second.folder)
                    .join(".install")
                    .join("pack.zip.part")
            )
            .expect("other partial remains"),
            b"partial"
        );
        service.shutdown().await.expect("close service");
    }

    #[test]
    fn legacy_archive_cleanup_preserves_unknown_files() {
        let (_, home) = fixture("unknown-cache-file");
        let directory = home.root().join("data").join("modpacks");
        fs::create_dir_all(&directory).expect("directory");
        fs::write(directory.join("pack.zip"), b"cache").expect("archive");
        fs::write(directory.join("notes.txt"), b"preserve").expect("unknown file");
        remove_legacy_archives(&directory).expect("cleanup");
        assert!(!directory.join("pack.zip").exists());
        assert_eq!(
            fs::read(directory.join("notes.txt")).expect("preserved file"),
            b"preserve"
        );
    }
}
