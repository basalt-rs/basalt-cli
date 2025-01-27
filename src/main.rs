mod cli;
use std::{path::Path, process};

use bedrock::ConfigReadError;
use clap::Parser;
use cli::Cli;
use tokio::fs::File;

pub async fn verify(config_file: &Path) -> anyhow::Result<()> {
    let mut file = File::open(config_file).await?;
    let res = bedrock::Config::read_async(
        &mut file,
        config_file.file_name().map(|s| s.to_string_lossy()),
    )
    .await;

    match res {
        Ok(config) => config,
        Err(ConfigReadError::MalformedData(err)) => {
            eprintln!("{:?}", err);
            process::exit(1);
        }
        err => err?,
    };

    // TODO: More detailed verification
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        cli::SubCmd::Verify { config_file } => verify(&config_file).await?,
        cli::SubCmd::Build { .. } => {
            todo!();
        }
        cli::SubCmd::Run { .. } => {
            todo!();
        }
    }
    Ok(())
}
