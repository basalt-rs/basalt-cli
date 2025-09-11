use std::{net::Ipv4Addr, ops::Sub};

use anyhow::{bail, Context};
use clap::Subcommand;

mod login;

#[derive(Clone, Debug, Subcommand, PartialEq, Eq, Hash)]
pub enum SubCmd {
    Login {
        #[arg(short, long)]
        code: Option<String>,
        #[arg(long)]
        host: Option<Ipv4Addr>,
        #[arg(short, long)]
        username: Option<String>,
        #[arg(short, long)]
        password: Option<String>,
    },
}

impl SubCmd {
    /// Validate that the provided arguments are correct
    pub fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Handle the Auth subcommand
pub async fn handle(subcommand: SubCmd) -> anyhow::Result<()> {
    subcommand
        .validate()
        .context("Failed to validate command")?;
    match subcommand {
        SubCmd::Login {
            code,
            host,
            username,
            password,
        } => login::handle(code, host, username, password).await,
    }
    .context("Failed to handle auth subcommand")
}
