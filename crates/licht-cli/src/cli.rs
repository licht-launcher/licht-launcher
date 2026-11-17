use clap::Parser;

use crate::commands::Command;

#[derive(Debug, Parser)]
#[command(about = "Install and launch game versions", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
