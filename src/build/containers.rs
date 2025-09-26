use lazy_static::lazy_static;
use regex::Regex;
use tempdir::TempDir;
use tokio_stream::StreamExt;

use anyhow::Context;
use bedrock::Config;
use colored::Colorize;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_process_stream::ProcessLineStream;
use tokio_tar::Archive;

use crate::cli::ContainerBackend;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref ANSI_REGEX: Regex = Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap();
}

pub async fn build_container_image(
    tar_bytes: Vec<u8>,
    tag: String,
    container_backend: ContainerBackend,
) -> anyhow::Result<()> {
    match container_backend {
        ContainerBackend::Docker => {
            let docker = bollard::Docker::connect_with_local_defaults()
                .context("Failed to connect to docker")?;
            let stream = docker.build_image(
                bollard::image::BuildImageOptions {
                    dockerfile: "Dockerfile",
                    t: &tag,
                    rm: true,
                    ..Default::default()
                },
                None,
                Some(tar_bytes.into()),
            );

            let prefix = "[BUILD]".blue();
            // Process the stream
            tokio::pin!(stream);
            while let Some(item) = stream.next().await {
                let msg = item.context("Failed to perform docker build")?;
                if let Some(stream) = msg.stream {
                    let clean = stream.replace('\r', ""); // strip carriage returns from progress bars
                    for line in clean.split('\n') {
                        // remove trailing \n but keep empty lines
                        let trimmed = line.trim_end_matches('\n');
                        if !is_only_formatting_or_whitespace(trimmed) {
                            println!("{} {}", prefix, trimmed);
                        }
                    }
                }
            }
            Ok(())
        }
        ContainerBackend::Podman => {
            ensure_podman_accessible()
                .await
                .context("Failed to validate that Podman was accessible")?;
            let tmp_dir = TempDir::new("basalt-build").context("Failed to create tempdir")?;
            // Unpack tar bytes to temporary directory where we will run `podman build`
            let mut ar = Archive::new(tar_bytes.as_slice());
            ar.unpack(&tmp_dir.path())
                .await
                .context("Failed to unpack tar to temporary directory")?;
            // Build command and convert to stream
            let mut build_cmd = Command::new("podman");
            build_cmd
                .arg("build")
                .arg("-t")
                .arg(tag)
                .arg(tmp_dir.path());
            let mut stream = ProcessLineStream::try_from(build_cmd)
                .context("Failed to create process stream from command")?;

            let prefix = "[BUILD]".blue();

            // Grab stdout and stderr for better perf
            let mut stdout = tokio::io::stdout();
            let mut stderr = tokio::io::stderr();
            while let Some(item) = stream.next().await {
                if let Some(out) = item.stdout() {
                    stdout
                        .write_all(format!("{} {}\n", prefix, out.clear()).as_bytes())
                        .await
                        .context("Failed to write to STDOUT")?;
                }
                if let Some(err) = item.stderr() {
                    stderr
                        .write_all(format!("{} {}\n", prefix, err).as_bytes())
                        .await
                        .context("Failed to write to STDERR")?;
                }
            }
            Ok(())
        }
    }
}

/// Based on the config, determine which tag to use
pub fn get_server_tag(cfg: &Config) -> String {
    let needs_scripting = !cfg.integrations.event_handlers.is_empty();
    let needs_webhooks = !cfg.integrations.webhooks.is_empty();
    let variant = if needs_scripting && needs_webhooks {
        "full"
    } else if needs_scripting {
        "scripting"
    } else if needs_webhooks {
        "webhooks"
    } else {
        "minimal"
    };
    format!("{APP_VERSION}-{variant}")
}

pub async fn ensure_podman_accessible() -> anyhow::Result<()> {
    Command::new("podman")
        .output()
        .await
        .context("Failed to spawn Podman command")?;
    Ok(())
}

fn strip_ansi(s: &str) -> String {
    // Regex for ANSI escape sequences
    ANSI_REGEX.replace_all(s, "").into_owned()
}

fn is_only_formatting_or_whitespace(s: &str) -> bool {
    let stripped = strip_ansi(s);
    stripped.chars().all(|c| {
        c.is_whitespace()
            || c.is_control()
            || matches!(c, '\u{200B}'..='\u{200F}'
                   | '\u{202A}'..='\u{202E}'
                   | '\u{2060}'..='\u{206F}'
                   | '\u{FEFF}')
    })
}
