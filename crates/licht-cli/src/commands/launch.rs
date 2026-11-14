use std::path::PathBuf;

use clap::Args;
use licht_core::{
    CoreError, LaunchEnvironment, OutputStream, SharedCache, configured_java, game_cache,
    global_launch, installed_java, instance_version_for_launch, is_version_ready, list_instances,
    load_instance, load_resolved_version_json, load_settings, log_codec, offline_account,
    parse_version, prepare_offline_launch, probe_java_major, required_java_major,
    resolved_java_path, resolved_launch, run_game,
};
use tokio::sync::mpsc;

use super::instances::{instance_refs, resolve_error, resolve_instance};
use crate::host::host_environment;

#[derive(Debug, Args)]
pub struct LaunchArgs {
    /// Installed version to launch.
    #[arg(
        long = "version",
        value_name = "ID",
        value_parser = clap::builder::NonEmptyStringValueParser::new(),
        conflicts_with = "instance",
        required_unless_present = "instance"
    )]
    pub version_id: Option<String>,
    /// Saved instance folder or unique display name.
    #[arg(
        long = "instance",
        value_name = "NAME_OR_FOLDER",
        value_parser = clap::builder::NonEmptyStringValueParser::new(),
        conflicts_with_all = ["version_id", "game_directory"],
        required_unless_present = "version_id"
    )]
    pub instance: Option<String>,
    /// Offline username for the game session.
    #[arg(
        long,
        value_name = "NAME",
        value_parser = clap::builder::NonEmptyStringValueParser::new()
    )]
    pub username: String,
    /// Use a custom Java executable.
    #[arg(long, value_name = "PATH")]
    pub java: Option<PathBuf>,
    /// Replace the default instance directory. Incompatible with `--instance`.
    #[arg(long = "game-dir", value_name = "PATH", conflicts_with = "instance")]
    pub game_directory: Option<PathBuf>,
    /// Use a custom cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache: Option<PathBuf>,
}

pub async fn launch(args: LaunchArgs) -> licht_core::Result<i32> {
    let pool = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&pool).await?,
    };
    let settings = load_settings(&pool).await?;
    let database = licht_core::DatabaseService::start(pool).await?;
    let launch_result: licht_core::Result<_> = async {
        let (version_id, instance, profile) = match &args.instance {
            Some(query) => {
                let root = cache.instances_dir()?;
                let entries = list_instances(&database.client(), &root).await?;
                let refs = instance_refs(&entries);
                let resolved =
                    resolve_instance(query, &refs).map_err(|error| resolve_error(query, error))?;
                let dir = cache.instance_dir(&resolved.folder)?;
                let profile = load_instance(&database.client(), &dir).await?;
                let version_id = instance_version_for_launch(&profile)?.to_string();
                (version_id, dir, Some(profile))
            }
            None => {
                let version_id = args
                    .version_id
                    .clone()
                    .ok_or_else(|| CoreError::Io(std::io::Error::other("version is required")))?;
                let instance = match args.game_directory {
                    Some(path) => path,
                    None => {
                        let path = cache.instance_dir(&version_id)?;
                        std::fs::create_dir_all(&path)?;
                        path
                    }
                };
                let profile = match load_instance(&database.client(), &instance).await {
                    Ok(profile) => Some(profile),
                    Err(CoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                        None
                    }
                    Err(error) => return Err(error),
                };
                (version_id, instance, profile)
            }
        };

        let environment = host_environment()?;
        require_installed_version(&cache, &version_id, &environment)?;
        let json = match &args.instance {
            Some(_) => load_resolved_version_json(&cache, &version_id)?,
            None => std::fs::read_to_string(cache.version_json(&version_id)?)?,
        };
        let version = parse_version(&json)?;
        let required = required_java_major(&version_id, &version);
        let profile_java = profile.as_ref().and_then(resolved_java_path);
        let custom_java = args.java.is_some() || profile_java.is_some();
        let java = match args.java {
            Some(path) => path,
            None => match profile_java {
                Some(path) => path,
                None => match configured_java(&settings, required) {
                    Some(path) => path,
                    None => installed_java(&cache, &version_id, &version, &environment)?,
                },
            },
        };
        let major = if custom_java {
            probe_java_major(&java)?
        } else {
            required
        };
        let codec = log_codec(major);
        let account = offline_account(&args.username)?;
        let global = global_launch(&settings);
        let launch_options = profile
            .as_ref()
            .map(|profile| resolved_launch(profile, &global))
            .unwrap_or(global);
        let launch = prepare_offline_launch(
            &database.client(),
            &java,
            &cache,
            &version_id,
            &version,
            &environment,
            &account,
            &instance,
            &launch_options,
        )
        .await?;
        Ok((launch, codec))
    }
    .await;
    database.shutdown().await?;
    let (launch, codec) = launch_result?;

    let (output, mut incoming) = mpsc::channel(32);
    let appdata = launch.appdata.map(|path| path.display().to_string());
    let command = launch.command;
    let game_directory = launch.game_directory;
    let running = tokio::spawn(async move {
        let mut env = Vec::new();
        if let Some(path) = &appdata {
            env.push(("APPDATA", path.as_str()));
        }
        run_game(&command, Some(&game_directory), &env, codec, output, None).await
    });
    while let Some(line) = incoming.recv().await {
        match line.stream {
            OutputStream::Stdout => println!("{}", line.line),
            OutputStream::Stderr => eprintln!("{}", line.line),
        }
    }
    let exit = running
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("launch task failed")))??;
    Ok(exit.code.unwrap_or(1))
}

fn require_installed_version(
    cache: &SharedCache,
    version_id: &str,
    environment: &LaunchEnvironment,
) -> licht_core::Result<()> {
    if is_version_ready(cache, version_id, environment) {
        return Ok(());
    }
    Err(CoreError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "version `{version_id}` is not installed; run `licht-cli install --version {version_id}` first"
        ),
    )))
}

#[cfg(test)]
mod tests {
    use super::require_installed_version;
    use licht_core::{Arch, LaunchEnvironment, OsName, SharedCache};
    use std::collections::BTreeMap;

    fn environment() -> LaunchEnvironment {
        LaunchEnvironment {
            os: OsName::Windows,
            arch: Arch::X86_64,
            os_version: String::new(),
            features: BTreeMap::new(),
        }
    }

    #[test]
    fn rejects_a_version_that_is_not_installed() {
        let root = std::env::temp_dir().join(format!(
            "licht-cli-launch-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp cache");
        let cache = SharedCache::at(&root);

        let error = require_installed_version(&cache, "1.20.1", &environment())
            .expect_err("missing install");
        let message = match error {
            licht_core::CoreError::Io(error) => error.to_string(),
            other => panic!("expected I/O error, got {other}"),
        };
        assert!(message.contains("version `1.20.1` is not installed"));
        assert!(message.contains("licht-cli install --version 1.20.1"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
