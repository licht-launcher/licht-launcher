mod cli;
mod commands;
mod error;
mod host;
mod progress;

use clap::Parser;

use cli::Cli;
use commands::{Command, InstancesCommand, install, instances_list, launch, versions};
use error::report;

fn main() {
    licht_core::load_env_files();
    let cli = Cli::parse();
    let code = match runtime() {
        Ok(runtime) => match runtime.block_on(run(cli)) {
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

async fn run(cli: Cli) -> licht_core::Result<i32> {
    match cli.command {
        Command::Launch(args) => launch(args).await,
        Command::Versions(args) => versions(args).await,
        Command::Install(args) => {
            install(args).await?;
            Ok(0)
        }
        Command::Instances(InstancesCommand::List(args)) => instances_list(args).await,
    }
}

fn runtime() -> licht_core::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?)
}
