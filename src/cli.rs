use std::{net::Ipv4Addr, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

fn default_config() -> &'static std::ffi::OsStr {
    std::ffi::OsStr::new("basalt.toml")
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum ContainerBackend {
    Docker,
    Podman,
}

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
    Init {
        #[arg()]
        path: Option<PathBuf>,
    },
    /// Build the docker file based on a given configuration file
    Build {
        /// Specifies tag for docker image. Not recommended unless you're familiar with Docker.
        #[arg(short, long)]
        tag: Option<String>,
        /// Path to output tarball
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The configuration file to build
        config_file: PathBuf,
        /// The backend to use to build container
        #[arg(long, value_enum, default_value_t = ContainerBackend::Docker)]
        container_backend: ContainerBackend,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Build the docker file based on a given configuration file and then run it using docker.
    Run {
        /// The configuration file to build
        config_file: PathBuf,
    },
    /// Render the logins into a printable packet PDF
    RenderLogins {
        /// Output file for the PDF (`.pdf` is optional).  If not specified, will get name from the
        /// config file used
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Path to a template to use, if not specified uses the default template.  Most of the
        /// time, this is not necessary.
        #[arg(short, long)]
        template: Option<PathBuf>,
        /// Config file from which to generate the PDF
        #[arg(default_value = default_config())]
        config_file: PathBuf,
    },
    /// Render the configuration into a printable packet PDF
    Render {
        /// Output file for the PDF (`.pdf` is optional).  If not specified, will get name from the
        /// config file used
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Path to a template to use, if not specified uses the default template.  Most of the
        /// time, this is not necessary.
        #[arg(short, long)]
        template: Option<PathBuf>,
        /// Config file from which to generate the PDF
        #[arg(default_value = default_config())]
        config_file: PathBuf,
    },
    /// Generate the game code for your computer on this network
    GameCode {
        /// Configuration for which to generate the code.  Used for getting port number
        #[arg(short, long, default_value = default_config(), conflicts_with = "port")]
        config: PathBuf,
        /// Port for which to generate the code
        #[arg(short, long, conflicts_with = "config")]
        port: Option<u16>,
        /// IPv4 for which to generate the code.  If not specified, attempts to automatically
        /// determine the IP address
        ip: Option<Ipv4Addr>,
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
