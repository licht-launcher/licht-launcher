mod install;
pub mod instances;
mod launch;
mod modpacks;
mod versions;

pub use install::{InstallArgs, install};
pub use instances::{InstancesCommand, instances_list};
pub use launch::{LaunchArgs, launch};
pub use modpacks::{ModpacksCommand, modpacks_install, modpacks_search, modpacks_versions};
pub use versions::{VersionsArgs, versions};

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List versions available in the official manifest.
    Versions(VersionsArgs),
    /// Install a version and its required Java runtime.
    Install(InstallArgs),
    /// Launch an installed version or a saved instance.
    Launch(LaunchArgs),
    /// Inspect saved instances.
    #[command(subcommand)]
    Instances(InstancesCommand),
    /// Search and install modpacks.
    #[command(subcommand)]
    Modpacks(ModpacksCommand),
}
