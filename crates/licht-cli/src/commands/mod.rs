mod install;
pub mod instances;
mod java;
mod launch;
mod modpacks;
mod repair;
pub mod versions;

pub use install::{InstallArgs, install};
pub use instances::{
    InstancesCommand, instances_create, instances_delete, instances_list, instances_rename,
};
pub use java::{JavaCommand, java_detect};
pub use launch::{LaunchArgs, launch};
pub use modpacks::{ModpacksCommand, modpacks_install, modpacks_search, modpacks_versions};
pub use repair::{RepairArgs, repair};
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
    /// Inspect Java runtimes.
    #[command(subcommand)]
    Java(JavaCommand),
    /// Search and install modpacks.
    #[command(subcommand)]
    Modpacks(ModpacksCommand),
    /// Repair an instance installation.
    Repair(RepairArgs),
}
