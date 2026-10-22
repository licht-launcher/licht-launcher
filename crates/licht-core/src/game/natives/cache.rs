use std::fs::{self, File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};

use super::{extract_natives, is_shared_library, native_libraries};
use crate::hashes::{file_matches_sha1, hex_encode, sha1_hex};
use crate::path_component::path_component_safe;
use crate::store::repositories::natives::{NativeGeneration, NativeOutput};
use crate::{
    Arch, CoreError, DatabaseClient, DownloadChecksum, LaunchEnvironment, Result, SharedCache,
    Version,
};

pub(crate) async fn prepare(
    database: &DatabaseClient,
    game: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
) -> Result<PathBuf> {
    let game = SharedCache::at(std::path::absolute(game.root())?);
    let fingerprint = fingerprint(&game, version, environment)?;
    let root = game.natives_dir(version_id)?;
    let platform_directory = root.join(platform(environment));
    let parent = platform_directory;
    let destination = crate::transfer::download::destination_lock(&parent.join(&fingerprint));
    let _destination = destination.lock().await;
    let directories = vec![game.root().join("natives"), root, parent.clone()];
    let lock_path = parent.join(format!(".lock-{fingerprint}"));
    let _lock = tokio::task::spawn_blocking(move || -> Result<File> {
        for directory in directories {
            fs::create_dir_all(&directory)?;
            let metadata = fs::symlink_metadata(directory)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(CoreError::CachePath);
            }
        }
        let path = lock_path;
        if metadata(&path)?.is_some_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(CoreError::CachePath);
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        lock.lock()?;
        Ok(lock)
    })
    .await
    .map_err(join_error)??;
    if let Some(generation) = database
        .native_generation(&game, version_id, &fingerprint)
        .await?
    {
        let cached_parent = parent.clone();
        if let Some(path) =
            tokio::task::spawn_blocking(move || cached_directory(&cached_parent, &generation))
                .await
                .map_err(join_error)??
        {
            return Ok(path);
        }
    }
    let build_game = game.clone();
    let build_parent = parent.clone();
    let build_version = version.clone();
    let build_environment = environment.clone();
    let generation = tokio::task::spawn_blocking(move || {
        build_generation(
            &build_game,
            &build_parent,
            &build_version,
            &build_environment,
        )
    })
    .await
    .map_err(join_error)??;
    let path = parent.join(&generation.generation);
    if let Err(error) = database
        .store_native_generation(&game, version_id, &fingerprint, generation)
        .await
    {
        return Err(cleanup_failed(&path, error));
    }
    Ok(path)
}

fn fingerprint(
    game: &SharedCache,
    version: &Version,
    environment: &LaunchEnvironment,
) -> Result<String> {
    let mut inputs = Vec::new();
    for library in native_libraries(version, environment) {
        game.library(&library.artifact.path)?;
        DownloadChecksum::Sha1(library.artifact.sha1.clone()).validate()?;
        inputs.push(serde_json::json!({"path":library.artifact.path,"sha1":library.artifact.sha1.to_ascii_lowercase(),"exclude":library.exclude}));
    }
    let bytes = serde_json::to_vec(
        &serde_json::json!({"layout":1,"platform":platform(environment),"inputs":inputs}),
    )
    .map_err(std::io::Error::other)?;
    Ok(hex_encode(Sha256::digest(bytes).as_slice()))
}

fn platform(environment: &LaunchEnvironment) -> String {
    let arch = match environment.arch {
        Arch::X86 => "x86",
        Arch::X86_64 => "x86_64",
    };
    format!("{}-{arch}", environment.os.mojang_name())
}

fn metadata(path: &Path) -> Result<Option<Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn safe_name(name: &str) -> bool {
    !name.contains(['/', '\\']) && path_component_safe(name) && is_shared_library(Path::new(name))
}

fn stamp(metadata: &Metadata) -> Result<String> {
    Ok(match metadata.modified()?.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}:{}", duration.as_secs(), duration.subsec_nanos()),
        Err(error) => format!(
            "-{}:{}",
            error.duration().as_secs(),
            error.duration().subsec_nanos()
        ),
    })
}

fn cached_directory(parent: &Path, generation: &NativeGeneration) -> Result<Option<PathBuf>> {
    if generation.generation.len() != 32
        || !generation
            .generation
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(None);
    }
    let path = parent.join(&generation.generation);
    let Some(directory) = metadata(&path)? else {
        return Ok(None);
    };
    if !directory.is_dir() || directory.file_type().is_symlink() {
        return Ok(None);
    }
    // The JVM may add JNA/LWJGL/Netty work files here; only managed outputs define cache validity.
    for output in &generation.files {
        if !safe_name(&output.name) {
            return Ok(None);
        }
        let Some(file) = metadata(&path.join(&output.name))? else {
            return Ok(None);
        };
        if !file.is_file()
            || file.file_type().is_symlink()
            || i64::try_from(file.len()).map_err(std::io::Error::other)? != output.size
            || stamp(&file)? != output.modified
        {
            return Ok(None);
        }
    }
    Ok(Some(path))
}

fn build_generation(
    game: &SharedCache,
    parent: &Path,
    version: &Version,
    environment: &LaunchEnvironment,
) -> Result<NativeGeneration> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|error| std::io::Error::other(error.to_string()))?;
    let generation = hex_encode(&nonce);
    let staging = parent.join(format!(".tmp-{generation}"));
    fs::create_dir(&staging)?;
    let result = (|| {
        let mut sources = Vec::new();
        for library in native_libraries(version, environment) {
            let source = game.library(&library.artifact.path)?;
            let before = fs::metadata(&source)?;
            if !file_matches_sha1(&source, &library.artifact.sha1)? {
                return Err(CoreError::Sha1Mismatch {
                    expected: library.artifact.sha1.clone(),
                    actual: sha1_hex(&fs::read(source)?),
                });
            }
            sources.push((source, before.len(), stamp(&before)?));
        }
        extract_natives(game, version, environment, &staging)?;
        for (source, size, modified) in sources {
            let after = fs::metadata(source)?;
            if after.len() != size || stamp(&after)? != modified {
                return Err(
                    std::io::Error::other("native archive changed during extraction").into(),
                );
            }
        }
        let mut files = Vec::new();
        for item in fs::read_dir(&staging)? {
            let item = item?;
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| CoreError::NativePath)?;
            if !safe_name(&name) || !item.file_type()?.is_file() {
                return Err(CoreError::NativePath);
            }
            // Persist output bytes before SQLite can commit a reference to this generation.
            OpenOptions::new()
                .write(true)
                .open(item.path())?
                .sync_all()?;
            let metadata = item.metadata()?;
            files.push(NativeOutput {
                name,
                size: i64::try_from(metadata.len()).map_err(std::io::Error::other)?,
                modified: stamp(&metadata)?,
            });
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        fs::rename(&staging, parent.join(&generation))?;
        Ok(NativeGeneration { generation, files })
    })();
    result.map_err(|error| cleanup_failed(&staging, error))
}

fn cleanup_failed(path: &Path, original: CoreError) -> CoreError {
    match fs::remove_dir_all(path) {
        Ok(()) => original,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => original,
        Err(error) => std::io::Error::other(format!(
            "native cache preparation failed: {original}; temporary cleanup failed: {error}"
        ))
        .into(),
    }
}

fn join_error(error: tokio::task::JoinError) -> CoreError {
    std::io::Error::other(format!("native cache worker failed: {error}")).into()
}

#[cfg(test)]
mod tests;
