use std::path::PathBuf;

use clap::Args;
use licht_core::{CoreError, SharedCache, fetch_version_manifest, game_cache, version_lines};

#[derive(Debug, Args)]
pub struct VersionsArgs {
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

pub async fn versions(args: VersionsArgs) -> licht_core::Result<i32> {
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&licht_core::initialize_launcher().await?).await?,
    };
    let client = reqwest::Client::builder()
        .timeout(licht_core::MANIFEST_REQUEST_TIMEOUT)
        .build()
        .map_err(CoreError::from)?;
    let manifest = fetch_version_manifest(&client, &cache).await?;
    for line in version_lines(&manifest) {
        println!("{line}");
    }
    Ok(0)
}
