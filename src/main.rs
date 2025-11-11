mod build;
mod cli;
mod init;
use std::{ffi::OsStr, path::Path, process};

use ansi_term::{
    Colour::{Blue, Green, Red, Yellow},
    Style,
};
use anyhow::Context;
use bedrock::{render::typst::PdfGenerationError, Config, ConfigReadError};
use build::build_with_output;
use clap::Parser;
use cli::Cli;
use tokio::{fs::File, io::BufWriter};

use crate::cli::Template;

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

macro_rules! error {
    ($format: literal $($tt: tt)*) => {
        eprintln!(
            concat!("{}: ", $format),
            Red.bold().paint("Error")
            $($tt)*
        )
    }
}

macro_rules! warning {
    ($format: literal $($tt: tt)*) => {
        eprintln!(
            concat!("{}: ", $format),
            Yellow.bold().paint("Warning")
            $($tt)*
        )
    }
}

macro_rules! success {
    ($format: literal $($tt: tt)*) => {
        eprintln!(
            concat!("{}: ", $format),
            Green.bold().paint("Success")
            $($tt)*
        )
    }
}

async fn render_pdf(config: Config, template: Template, output: &Path) -> anyhow::Result<()> {
    let mut file = BufWriter::new(
        File::create(&output)
            .await
            .with_context(|| format!("opening for writing: {}", output.display()))?,
    );
    let res = config.generate_pdf_async(&mut file, template).await;

    match res {
        Ok(_) => {}
        Err(PdfGenerationError::MissingTypstCommand) => {
            error!(
                "The 'typst' command must be installed and in your PATH in order to render a PDF."
            );
            return Ok(());
        }
        Err(PdfGenerationError::TypstError) => {
            // Typst has already errored, so we don't need to
            return Ok(());
        }
        Err(PdfGenerationError::IoError(e)) => {
            Err(e).context("generating pdf")?;
        }
    }
    success!("Rendered PDF to {}", output.display());
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        cli::SubCmd::Verify { config_file } => verify(&config_file).await?,
        cli::SubCmd::Init { path } => init::handle(path).await?,
        cli::SubCmd::Build {
            tag,
            output,
            config_file,
            container_backend,
        } => build_with_output(&output, &config_file, tag, container_backend).await?,
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
            let output = output
                .unwrap_or_else(|| {
                    config_file
                        .file_name()
                        .expect("This would have failed when opening the file")
                        .into()
                })
                .with_extension("pdf");
            render_pdf(config, template, &output).await?;
        }
        cli::SubCmd::RenderLogins {
            output,
            config_file,
            template,
        } => {
            warning!("The render-logins command is deprecated and scheduled for removal.");
            eprintln!(
                "         Use {}",
                Style::new().bold().paint(format!(
                    "{} render --template {}",
                    std::env::args().next().unwrap(),
                    if template == Template::Logins {
                        "logins"
                    } else {
                        "<template>"
                    }
                )),
            );

            let mut file = File::open(&config_file)
                .await
                .context("opening config file")?;
            let config = bedrock::Config::read_async(
                &mut file,
                config_file.file_name().and_then(OsStr::to_str),
            )
            .await
            .context("loading config")?;
            let output = output
                .unwrap_or_else(|| {
                    config_file
                        .file_name()
                        .expect("This would have failed when opening the file")
                        .into()
                })
                .with_extension("pdf");
            render_pdf(config, template, &output).await?;
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
