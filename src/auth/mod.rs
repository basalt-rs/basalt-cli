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
        addr: Option<String>,
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
            addr,
            username,
            password,
        } => login::handle(code, addr, username, password).await,
    }
    .context("Failed to handle auth subcommand")
}
