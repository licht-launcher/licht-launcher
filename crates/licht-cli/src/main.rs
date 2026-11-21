use clap::Parser;

use licht_cli::cli::Cli;
use licht_cli::commands::{
    Command, InstancesCommand, JavaCommand, install, instances_list, java_detect, launch, versions,
};
use licht_cli::error::report;
use licht_cli::logging;

fn main() {
    licht_core::load_env_files();
    let cli = Cli::parse();
    if let Err(error) = logging::init(cli.verbose) {
        report(&error);
        std::process::exit(1);
    }
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
        Command::Java(JavaCommand::Detect(args)) => java_detect(args).await,
    }
}

fn runtime() -> licht_core::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?)
}
