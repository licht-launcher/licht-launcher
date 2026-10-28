use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::progress::{StartupPhase, StartupReporter};
use super::{Relocation, database, files, migration, relocate};
use crate::{CoreError, Result, SharedCache};

// Operational locks and update requests belong to the profile, not its content.
const CONTENT_CACHES: &[&str] = &["icons", "content-metadata"];

#[derive(Serialize, Deserialize)]
struct DirectoryState {
    active: PathBuf,
    pending: Option<PendingMove>,
}

#[derive(Serialize, Deserialize)]
struct PendingMove {
    destination: PathBuf,
    copied: bool,
}

fn state_path(home: &SharedCache) -> PathBuf {
    home.root().join("content-directory.json")
}

fn write_state(home: &SharedCache, state: &DirectoryState) -> Result<()> {
    fs::create_dir_all(home.root())?;
    let path = state_path(home);
    let temporary = path.with_extension("json.tmp");
    let mut file = fs::File::create(&temporary)?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec(state).map_err(CoreError::Settings)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}

fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        || path.file_name().is_none_or(|name| name != "data")
    {
        return Err(CoreError::SettingsDirectory);
    }
    Ok(())
}

fn mappings(source: &SharedCache, destination: &SharedCache) -> Result<Vec<Relocation>> {
    let mut mappings = vec![
        Relocation {
            source: source.root().to_path_buf(),
            destination: destination.root().to_path_buf(),
        },
        Relocation {
            source: source.instances_dir()?,
            destination: destination.instances_dir()?,
        },
    ];
    for directory in CONTENT_CACHES {
        mappings.push(Relocation {
            source: source.cache_dir()?.join(directory),
            destination: destination.cache_dir()?.join(directory),
        });
    }
    Ok(mappings)
}

fn validate_roots(home: &SharedCache, mappings: &[Relocation]) -> Result<()> {
    let profile = fs::canonicalize(home.root())?;
    for mapping in mappings {
        validate_no_links(&mapping.source)?;
        validate_no_links(&mapping.destination)?;
    }
    let roots = mappings
        .iter()
        .map(|mapping| {
            Ok((
                canonical_directory(&mapping.source)?,
                canonical_directory(&mapping.destination)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    for (_, destination) in &roots {
        if profile.starts_with(destination) {
            return Err(CoreError::SettingsDirectory);
        }
        for (other_source, _) in &roots {
            if destination.starts_with(other_source) || other_source.starts_with(destination) {
                return Err(CoreError::SettingsDirectory);
            }
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(CoreError::CachePath)?;
            Ok(canonical_directory(parent)?.join(path.file_name().ok_or(CoreError::CachePath)?))
        }
        Err(error) => Err(error.into()),
    }
}

fn validate_no_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(CoreError::CachePath),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) async fn apply(home: &SharedCache, reporter: &StartupReporter) -> Result<()> {
    let desired = crate::game_cache(home)?;
    let mut state: DirectoryState = match fs::read(state_path(home)) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(CoreError::Settings)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Existing installations already use their configured directory.
            write_state(
                home,
                &DirectoryState {
                    active: desired.root().to_path_buf(),
                    pending: None,
                },
            )?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    validate_path(&state.active)?;
    if state.active == desired.root()
        && state.pending.as_ref().is_none_or(|pending| !pending.copied)
    {
        if state.pending.is_some() {
            state.pending = None;
            write_state(home, &state)?;
        }
        return Ok(());
    }
    if state.pending.as_ref().is_none_or(|pending| !pending.copied) {
        state.pending = Some(PendingMove {
            destination: desired.root().to_path_buf(),
            copied: false,
        });
        write_state(home, &state)?;
    }
    let pending = state.pending.as_ref().ok_or(CoreError::CachePath)?;
    validate_path(&pending.destination)?;
    if pending.destination != desired.root() {
        return Err(std::io::Error::other(
            "finish the pending content-directory move before choosing another destination",
        )
        .into());
    }
    let source = SharedCache::at(&state.active);
    let destination = SharedCache::at(&pending.destination);
    if !pending.copied && !source.root().is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the previous content directory is unavailable",
        )
        .into());
    }
    if pending.copied && !destination.root().is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the copied content directory is unavailable",
        )
        .into());
    }
    let mappings = mappings(&source, &destination)?;
    validate_roots(home, &mappings)?;
    reporter.phase(StartupPhase::MigratingData)?;
    if !pending.copied {
        files::copy_tree_with_progress(source.root(), destination.root(), reporter)?;
        migration::copy_instances(
            &source.instances_dir()?,
            &destination.instances_dir()?,
            &home.root().join("cache/content-move-profiles"),
            &mappings,
            reporter,
        )?;
        for mapping in mappings.iter().skip(2) {
            files::copy_tree_with_progress(&mapping.source, &mapping.destination, reporter)?;
        }
        state.pending.as_mut().ok_or(CoreError::CachePath)?.copied = true;
        write_state(home, &state)?;
    }
    reporter.phase(StartupPhase::MigratingDatabase)?;
    database::remap(home, &mappings).await?;
    let mut settings = crate::load_settings(home)?;
    for path in [
        &mut settings.java25,
        &mut settings.java21,
        &mut settings.java17,
        &mut settings.java8,
    ]
    .into_iter()
    .flatten()
    {
        *path = relocate(Path::new(path), &mappings)
            .to_string_lossy()
            .into_owned();
    }
    crate::save_settings(home, &settings)?;
    // The checkpoint keeps the original roots until cleanup completes. Remapping is idempotent.
    for mapping in &mappings {
        remove_verified(&mapping.source, &mapping.destination, &mappings, reporter)?;
    }
    let staging = home.root().join("cache/content-move-profiles");
    if staging.exists() {
        fs::remove_dir_all(staging)?;
    }
    state.active = destination.root().to_path_buf();
    state.pending = None;
    write_state(home, &state)?;
    tracing::info!("Content directory move completed");
    Ok(())
}

fn remove_verified(
    source: &Path,
    destination: &Path,
    mappings: &[Relocation],
    reporter: &StartupReporter,
) -> Result<()> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    reporter.checkpoint()?;
    validate_no_links(destination)?;
    if metadata.is_dir() {
        for item in fs::read_dir(source)? {
            let item = item?;
            remove_verified(
                &item.path(),
                &destination.join(item.file_name()),
                mappings,
                reporter,
            )?;
        }
        fs::remove_dir(source)?;
    } else if metadata.is_file() {
        let legacy_profile = source
            .file_name()
            .is_some_and(|name| name == "instance.json")
            && source.parent().and_then(Path::parent) == Some(mappings[1].source.as_path());
        let matches = if legacy_profile {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(source)?).map_err(CoreError::Instance)?;
            if let Some(path) = value.get("javaPath").and_then(serde_json::Value::as_str) {
                value["javaPath"] = serde_json::Value::String(
                    relocate(Path::new(path), mappings)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            value
                == serde_json::from_slice::<serde_json::Value>(&fs::read(destination)?)
                    .map_err(CoreError::Instance)?
        } else {
            files::digest(source, reporter)? == files::digest(destination, reporter)?
        };
        if !matches {
            return Err(std::io::Error::other(
                "content changed during directory move; preserving source",
            )
            .into());
        }
        #[cfg(windows)]
        if metadata.permissions().readonly() {
            // Inherit the destination directory's writable flag only for source cleanup.
            let parent = destination.parent().ok_or(CoreError::CachePath)?;
            fs::set_permissions(source, fs::metadata(parent)?.permissions())?;
        }
        fs::remove_file(source)?;
    } else {
        return Err(CoreError::CachePath);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
