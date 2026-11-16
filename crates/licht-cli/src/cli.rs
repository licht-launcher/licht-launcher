use clap::Parser;

use crate::commands::Command;

#[derive(Debug, Parser)]
#[command(about = "Install and launch game versions", version)]
pub struct Cli {
    /// Increase log verbosity (-v = debug, -vv = trace). Ignored when `RUST_LOG` is set.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: Command,
}
