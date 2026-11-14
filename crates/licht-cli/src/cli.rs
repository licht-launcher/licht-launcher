use clap::Parser;

use crate::commands::Command;

#[derive(Debug, Parser)]
#[command(about = "Install and launch game versions", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser, error::ErrorKind};

    use super::Cli;
    use crate::commands::{Command, InstancesCommand};

    #[test]
    fn parses_versions_options() {
        let cli = Cli::try_parse_from(["licht-cli", "versions", "--cache", "cache"])
            .expect("valid versions arguments");

        let Command::Versions(args) = cli.command else {
            panic!("versions command expected");
        };
        assert_eq!(args.cache.as_deref(), Some(std::path::Path::new("cache")));
    }

    #[test]
    fn parses_install_options() {
        let cli = Cli::try_parse_from([
            "licht-cli",
            "install",
            "--version",
            "1.20.1",
            "--java",
            "java",
        ])
        .expect("valid install arguments");

        let Command::Install(args) = cli.command else {
            panic!("install command expected");
        };
        assert_eq!(args.version_id, "1.20.1");
        assert_eq!(args.java.as_deref(), Some(std::path::Path::new("java")));
    }

    #[test]
    fn parses_launch_options() {
        let cli = Cli::try_parse_from([
            "licht-cli",
            "launch",
            "--version",
            "1.20.1",
            "--username",
            "Player",
            "--game-dir",
            "instance",
        ])
        .expect("valid launch arguments");

        let Command::Launch(args) = cli.command else {
            panic!("launch command expected");
        };
        assert_eq!(args.version_id.as_deref(), Some("1.20.1"));
        assert!(args.instance.is_none());
        assert_eq!(args.username, "Player");
        assert_eq!(
            args.game_directory.as_deref(),
            Some(std::path::Path::new("instance"))
        );
    }

    #[test]
    fn parses_launch_by_instance() {
        let cli = Cli::try_parse_from([
            "licht-cli",
            "launch",
            "--instance",
            "Survival",
            "--username",
            "Player",
        ])
        .expect("valid instance launch");

        let Command::Launch(args) = cli.command else {
            panic!("launch command expected");
        };
        assert_eq!(args.instance.as_deref(), Some("Survival"));
        assert!(args.version_id.is_none());
        assert!(args.game_directory.is_none());
    }

    #[test]
    fn parses_instances_list() {
        let cli = Cli::try_parse_from(["licht-cli", "instances", "list", "--cache", "cache"])
            .expect("valid instances list");

        let Command::Instances(InstancesCommand::List(args)) = cli.command else {
            panic!("instances list expected");
        };
        assert_eq!(args.cache.as_deref(), Some(std::path::Path::new("cache")));
    }

    #[test]
    fn launch_rejects_version_and_instance_together() {
        let error = Cli::try_parse_from([
            "licht-cli",
            "launch",
            "--version",
            "1.20.1",
            "--instance",
            "Survival",
            "--username",
            "Player",
        ])
        .expect_err("version and instance conflict");

        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn launch_rejects_instance_with_game_dir() {
        let error = Cli::try_parse_from([
            "licht-cli",
            "launch",
            "--instance",
            "Survival",
            "--game-dir",
            "elsewhere",
            "--username",
            "Player",
        ])
        .expect_err("instance and game-dir conflict");

        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn launch_requires_version_or_instance() {
        let error = Cli::try_parse_from(["licht-cli", "launch", "--username", "Player"])
            .expect_err("a launch target is required");

        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn install_requires_a_version() {
        let error = Cli::try_parse_from(["licht-cli", "install"]).expect_err("version is required");

        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
        assert!(error.to_string().contains("--version <ID>"));
    }

    #[test]
    fn install_rejects_an_empty_version() {
        let error = Cli::try_parse_from(["licht-cli", "install", "--version", ""])
            .expect_err("version cannot be empty");

        assert_eq!(error.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn versions_rejects_an_unknown_flag() {
        let error =
            Cli::try_parse_from(["licht-cli", "versions", "--nope"]).expect_err("unknown flag");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
        assert!(error.to_string().contains("--nope"));
    }

    #[test]
    fn help_is_available() {
        let error = Cli::try_parse_from(["licht-cli", "--help"]).expect_err("help exits parsing");

        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        assert!(error.to_string().contains("Usage: licht-cli <COMMAND>"));
    }

    #[test]
    fn version_is_available() {
        let error =
            Cli::try_parse_from(["licht-cli", "--version"]).expect_err("version exits parsing");

        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert!(error.to_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
