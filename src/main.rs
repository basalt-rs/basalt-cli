mod build;
mod cli;
use std::{ffi::OsStr, path::Path, process};

use ansi_term::Colour::{Blue, Green};
use anyhow::Context;
use bedrock::ConfigReadError;
use build::build_with_output;
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

fn make_game_code<const N: usize>(bytes: [u8; N]) -> String {
    let mut s = String::with_capacity(2 * N);
    for b in bytes {
        s.push(char::from((b >> 4) + b'a'));
        s.push(char::from((b & 0xf) + b'a'));
    }
    s
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        cli::SubCmd::Verify { config_file } => verify(&config_file).await?,
        cli::SubCmd::Build {
            tag,
            output,
            config_file,
        } => build_with_output(&output, &config_file, tag).await?,
        cli::SubCmd::Run { .. } => {
            todo!();
        }
        cli::SubCmd::Render {
            output,
            config_file,
            template,
        } => {
            let mut file = File::open(&config_file)
                .await
                .context("opening config file")?;
            let config = bedrock::Config::read_async(
                &mut file,
                config_file.file_name().and_then(OsStr::to_str),
            )
            .await
            .context("loading config")?;
            let template = if let Some(template) = template {
                Some(
                    tokio::fs::read_to_string(template)
                        .await
                        .context("reading config file")?,
                )
            } else {
                None
            };

            let output = output
                .unwrap_or(
                    config_file
                        .file_name()
                        .expect("This would have failed when opening the file")
                        .into(),
                )
                .with_extension("pdf");
            let pdf = config.render_pdf(template).context("creating pdf")?;
            tokio::fs::write(&output, pdf)
                .await
                .with_context(|| format!("saving pdf to {}", output.display()))?;
            println!("Rendered PDF to {}", output.display());
        }
        cli::SubCmd::GameCode { config, ip, port } => {
            let ip = if let Some(ip) = ip {
                ip
            } else {
                match local_ip_address::local_ip().context("getting local IP address")? {
                    std::net::IpAddr::V4(addr) => addr,
                    std::net::IpAddr::V6(_) => unreachable!(
                        "Unreachable according to the documentation of local_ip_address::local_ip"
                    ),
                }
            };

            let port = if let Some(port) = port {
                port
            } else {
                let mut file = File::open(&config).await.context("opening config file")?;
                let config = bedrock::Config::read_async(
                    &mut file,
                    config.file_name().and_then(OsStr::to_str),
                )
                .await
                .context("loading config")?;
                config.port
            };

            let mut x = [0; 6];
            x[..4].copy_from_slice(&ip.octets());
            x[4..].copy_from_slice(&port.to_be_bytes());

            let code = make_game_code(x);

            println!(
                "{} {} {} {}",
                Green.paint("Game code for address"),
                Blue.paint(format!("{}:{}", ip, port)),
                Green.paint("is"),
                Blue.paint(code)
            );
        }
    }
    Ok(())
}
