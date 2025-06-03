mod build;
mod cli;
mod init;
use std::{ffi::OsStr, path::Path, process, time::Duration};

use ansi_term::Colour::{Blue, Green};
use anyhow::Context;
use bedrock::{ConfigReadError, User};
use build::build_with_output;
use clap::Parser;
use cli::Cli;
use futures::{SinkExt, TryStreamExt};
use hdrhistogram::Histogram;
use reqwest_websocket::{CloseCode, Message, RequestBuilderExt};
use serde_json::json;
use tokio::{fs::File, process::Command, task::JoinSet, time::Instant};

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
        cli::SubCmd::Init { path } => init::handle(path).await?,
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
        cli::SubCmd::RenderLogins {
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
                    format!(
                        "{}-logins",
                        config_file
                            .with_extension("")
                            .file_name()
                            .expect("This would have failed when opening the file")
                            .to_string_lossy()
                    )
                    .into(),
                )
                .with_extension("pdf");
            let pdf = config.render_login_pdf(template).context("creating pdf")?;
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
        cli::SubCmd::Benchmark {
            port,
            ref config,
            ref server_binary,
        } => {
            dbg!(port, server_binary);

            let mut child = Command::new(server_binary)
                .arg("run")
                .arg(config)
                .args(["-p", &port.to_string()])
                .env("BASALT_SERVER_LOGGING", "basalt_server=trace")
                .spawn()?;

            let mut file = File::open(config).await?;
            let config = bedrock::Config::read_async(
                &mut file,
                config.file_name().map(|s| s.to_string_lossy()),
            )
            .await?;

            let mut joins = JoinSet::new();

            eprintln!("Waiting for server to start...");
            tokio::time::sleep(Duration::from_millis(1000)).await;

            for team in config.accounts.competitors.clone() {
                async fn x(team: User, port: u16) -> anyhow::Result<()> {
                    dbg!(&team);
                    let client = reqwest::Client::new();
                    let res = client
                        .post(format!("http://127.0.0.1:{}/auth/login", port))
                        .json(&json! {{
                            "username": team.name,
                            "password": team.password,
                        }})
                        .send()
                        .await?;
                    dbg!(res.status());
                    let data: serde_json::Value = res.json().await?;
                    dbg!(&data);
                    let serde_json::Value::String(token) = &data["token"] else {
                        anyhow::bail!("Token is not a string");
                    };
                    dbg!(token);
                    let ws = client
                        .get(format!("ws://127.0.0.1:{}/ws", port))
                        .header(reqwest::header::SEC_WEBSOCKET_PROTOCOL, token)
                        .upgrade()
                        .protocols([token])
                        .send()
                        .await?;
                    let mut ws = ws.into_websocket().await?;
                    let mut hist = Histogram::<u64>::new(2).unwrap();
                    let mut items = Vec::with_capacity(100);
                    for x in 0..100 {
                        let start = Instant::now();
                        ws.send(Message::Text(serde_json::to_string(&serde_json::json! {{
                            "id": x,
                            "kind": "run-test",
                            "language": "java",
                            "solution": r#"
                                public class Solution {
                                    public static void main(String[] args) {
                                        System.out.println("olleh");
                                    }
                                }
                                "#,
                            "problem": 0,
                        }})?))
                        .await?;
                        ws.try_next().await?;
                        let elapsed = start.elapsed();
                        hist += elapsed.as_millis() as u64; // It is unlikley that this will take more than 584542046 years...
                        items.push(elapsed.as_millis());
                    }
                    ws.close(CloseCode::Normal, None).await?;
                    for f in hist.iter_linear(100) {
                        let n = f.value_iterated_to();
                        let c = f.count_at_value();

                        eprintln!("n = {}, c = {}", n, c);
                    }
                    dbg!(items);
                    Ok(())
                }
                joins.spawn(async move { x(team, port).await });
            }

            dbg!("start");
            joins
                .join_all()
                .await
                .into_iter()
                .collect::<anyhow::Result<()>>()?;
            dbg!("end");

            child.kill().await?;

            child.wait().await?;
        }
    }
    Ok(())
}
