use std::path::PathBuf;

use clap::Args;
use licht_core::{
    ASSET_OBJECT_BASE, CacheCheck, CoreError, GameInstall, SharedCache, fetch_version_manifest,
    game_cache, install_game,
};
use tokio::sync::mpsc;

use crate::host::host_environment;
use crate::progress::drain_download_progress;

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Version to install.
    #[arg(
        long = "version",
        value_name = "ID",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub version_id: String,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
    /// Use a custom Java executable.
    #[arg(long, value_name = "PATH")]
    pub java: Option<PathBuf>,
}

pub async fn install(args: InstallArgs) -> licht_core::Result<()> {
    let settings_home = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&settings_home).await?,
    };
    let client = reqwest::Client::builder()
        .timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let manifest = fetch_version_manifest(&client, &cache).await?;
    let Some(entry) = manifest
        .versions
        .iter()
        .find(|version| version.id == args.version_id)
    else {
        return Err(CoreError::VersionMissing);
    };

    let (progress, mut incoming) = mpsc::channel(4096);
    let version_id = args.version_id;
    let version_url = entry.url.clone();
    let version_sha1 = entry.sha1.clone();
    let java = args.java;
    let environment = host_environment()?;
    let installing = tokio::spawn(async move {
        install_game(
            &client,
            &cache,
            &settings_home,
            GameInstall {
                version_id: &version_id,
                version_json_url: &version_url,
                version_sha1: &version_sha1,
                environment: &environment,
                asset_base: ASSET_OBJECT_BASE,
                java: java.as_deref(),
            },
            CacheCheck::Reuse,
            progress,
        )
        .await
    });
    drain_download_progress(&mut incoming).await;
    let java = installing
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
    println!("java {}", java.display());
    Ok(())
}
