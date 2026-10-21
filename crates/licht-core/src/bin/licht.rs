use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use licht_core::{
    ASSET_OBJECT_BASE, Arch, CoreError, GameInstall, LaunchEnvironment, OsName, OutputStream,
    SharedCache, configured_java, fetch_version_manifest, game_cache, global_launch, install_game,
    installed_java, load_instance, load_settings, log_codec, offline_account, parse_install_args,
    parse_launch_args, parse_version, parse_versions_args, prepare_offline_launch,
    probe_java_major, required_java_major, resolved_java_path, resolved_launch, run_game,
    version_lines,
};
use tokio::sync::mpsc;

fn main() {
    licht_core::load_env_files();
    let code = match runtime() {
        Ok(runtime) => match runtime.block_on(run()) {
            Ok(code) => code,
            Err(error) => {
                report(&error);
                1
            }
        },
        Err(error) => {
            report(&error);
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> licht_core::Result<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("launch") => launch(&args).await,
        Some("versions") => versions(&args).await,
        Some("install") => {
            install(&args).await?;
            Ok(0)
        }
        _ => Err(CoreError::LaunchArgs),
    }
}

fn report(error: &CoreError) {
    eprintln!("{error}");
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        eprintln!("  {cause}");
        source = cause.source();
    }
}

fn runtime() -> licht_core::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?)
}

async fn install(args: &[String]) -> licht_core::Result<()> {
    let args = parse_install_args(args)?;
    let settings_home = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&settings_home)?,
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
    let version_id = args.version_id.clone();
    let version_url = entry.url.clone();
    let version_sha1 = entry.sha1.clone();
    let java = args.java.clone();
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
            progress,
        )
        .await
    });
    let interactive = std::io::stdout().is_terminal();
    let mut previous_total = None;
    let mut last_draw = None;
    while let Some(progress) = incoming.recv().await {
        let complete = progress.total > 0 && progress.finished + progress.failed == progress.total;
        let phase_changed = previous_total.is_some_and(|total| total != progress.total);
        let due =
            last_draw.is_none_or(|drawn: Instant| drawn.elapsed() >= Duration::from_millis(100));
        let line = progress_line(progress.bytes_done, progress.bytes_total);
        if interactive {
            if !complete && !phase_changed && !due {
                continue;
            }
            if phase_changed {
                println!();
            }
            print!("\r{line}");
            let _ = std::io::stdout().flush();
            if complete {
                println!();
            }
        } else if complete || phase_changed || progress.finished % 100 == 0 {
            println!("{line}");
        } else {
            continue;
        }
        previous_total = Some(progress.total);
        last_draw = Some(Instant::now());
    }
    let java = installing
        .await
        .map_err(|_| CoreError::Io(std::io::Error::other("install task failed")))??;
    println!("java {}", java.display());
    Ok(())
}

async fn versions(args: &[String]) -> licht_core::Result<i32> {
    let args = parse_versions_args(args)?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&licht_core::initialize_launcher().await?)?,
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

async fn launch(args: &[String]) -> licht_core::Result<i32> {
    let args = parse_launch_args(args)?;
    let settings_home = licht_core::initialize_launcher().await?;
    let cache = match &args.cache {
        Some(root) => SharedCache::at(root),
        None => game_cache(&settings_home)?,
    };
    let settings = load_settings(&settings_home)?;
    let json = std::fs::read_to_string(cache.version_json(&args.version_id)?)?;
    let version = parse_version(&json)?;
    let environment = host_environment()?;
    let required = required_java_major(&args.version_id, &version);
    let instance = match args.game_directory {
        Some(path) => path,
        None => {
            let path = cache.instance_dir(&args.version_id)?;
            std::fs::create_dir_all(&path)?;
            path
        }
    };
    let database = licht_core::DatabaseService::start(settings_home.clone()).await?;
    let profile = match load_instance(&database.client(), &instance).await {
        Ok(profile) => Some(profile),
        Err(CoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            database.shutdown().await?;
            return Err(error);
        }
    };
    database.shutdown().await?;
    let profile_java = profile.as_ref().and_then(resolved_java_path);
    let custom_java = args.java.is_some() || profile_java.is_some();
    let java = match args.java {
        Some(path) => path,
        None => match profile_java {
            Some(path) => path,
            None => match configured_java(&settings, required) {
                Some(path) => path,
                None => installed_java(&cache, &args.version_id, &version, &environment)?,
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
        &java,
        &cache,
        &args.version_id,
        &version,
        &environment,
        &account,
        &instance,
        &launch_options,
    )?;

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

const PROGRESS_WIDTH: usize = 24;
/// Eight steps per cell, so a download between two cells still moves the bar.
const PROGRESS_STEPS: u64 = (PROGRESS_WIDTH * 8) as u64;
const PARTIAL_BLOCK: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
/// Same glyph as the filled cells, drawn gray so the track is not a taller shade.
const PENDING: &str = "\u{1b}[90m";
const RESET: &str = "\u{1b}[0m";

fn progress_line(done: u64, total: u64) -> String {
    let done = done.min(total);
    let steps = if total == 0 {
        0
    } else {
        (u128::from(done) * u128::from(PROGRESS_STEPS) / u128::from(total)) as usize
    };
    let steps = steps.min(PROGRESS_STEPS as usize);
    let full = steps / 8;
    let partial = steps % 8;
    let mut filled = "█".repeat(full);
    if partial > 0 {
        filled.push(PARTIAL_BLOCK[partial]);
    }
    let rest = PROGRESS_WIDTH - full - usize::from(partial > 0);
    let bar = if rest == 0 {
        format!("[{filled}]")
    } else {
        format!("[{filled}{PENDING}{}{RESET}]", "█".repeat(rest))
    };
    let percent = if total == 0 {
        0
    } else {
        u128::from(done) * 1000 / u128::from(total)
    };
    format!(
        "{bar} {percent}.{percent_frac}%  {done_mb}.{done_frac}MB/{total_mb}.{total_frac}MB",
        percent = percent / 10,
        percent_frac = percent % 10,
        done_mb = megabytes(done).0,
        done_frac = megabytes(done).1,
        total_mb = megabytes(total).0,
        total_frac = megabytes(total).1,
    )
}

fn megabytes(bytes: u64) -> (u64, u64) {
    let tenths = bytes.saturating_mul(10) / 1_000_000;
    (tenths / 10, tenths % 10)
}

fn host_environment() -> licht_core::Result<LaunchEnvironment> {
    let os = match std::env::consts::OS {
        "windows" => OsName::Windows,
        "linux" => OsName::Linux,
        "macos" => OsName::Osx,
        _ => return Err(CoreError::LaunchHost),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "x86" => Arch::X86,
        _ => return Err(CoreError::LaunchHost),
    };
    Ok(LaunchEnvironment {
        os,
        arch,
        os_version: String::new(),
        features: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::progress_line;

    #[test]
    fn an_empty_download_draws_an_empty_bar() {
        assert_eq!(
            progress_line(0, 0),
            "[\u{1b}[90m████████████████████████\u{1b}[0m] 0.0%  0.0MB/0.0MB"
        );
    }

    #[test]
    fn a_half_finished_download_fills_half_the_bar() {
        assert_eq!(
            progress_line(5_000_000, 10_000_000),
            "[████████████\u{1b}[90m████████████\u{1b}[0m] 50.0%  5.0MB/10.0MB"
        );
    }

    #[test]
    fn megabytes_keep_one_decimal() {
        assert_eq!(
            progress_line(50_300_000, 80_200_000),
            "[███████████████\u{1b}[90m█████████\u{1b}[0m] 62.7%  50.3MB/80.2MB"
        );
    }
}
