use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Subcommand, PartialEq, Eq, Hash)]
pub enum SubCmd {
    /// Verify that the configuration in a configuration file is correct without attempting to
    /// build the Dockerfile.
    ///
    /// - Exits with 0 if config file is valid
    /// - Exits with 1 if config file is not valid
    #[command(verbatim_doc_comment)]
    Verify {
        /// The configuration file to verify
        config_file: PathBuf,
    },
    /// Build the docker file based on a given configuration file
    Build {
        /// Specifies tag for docker image. Not recommended unless you're familiar with Docker.
        #[arg(short, long)]
        tag: Option<String>,
        /// Path to directory where generated artifacts should be placed
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The configuration file to build
        config_file: PathBuf,
    },
    /// Build the docker file based on a given configuration file and then run it using docker.
    Run {
        /// The configuration file to build
        config_file: PathBuf,
    },
}

/// CLI tool for generating and running the docker container needed for hosting a basalt
/// competition
#[derive(Clone, Debug, Parser, PartialEq, Eq, Hash)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: SubCmd,
}
