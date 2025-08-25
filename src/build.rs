use std::path::{Path, PathBuf};

use anyhow::Context;
use bedrock::Config;
use futures::StreamExt;
use lazy_static::lazy_static;
use tokio::{io::AsyncReadExt, task::JoinSet};
use tokio_tar::{Builder, Header};

const BASE_DOCKER_SRC: &str = include_str!("../data/basalt.Dockerfile");
const INSTALL_SRC: &str = include_str!("../data/install.sh");
const ENTRY_SRC: &str = include_str!("../data/entrypoint.sh");

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref tmpl: tera::Tera = {
        let mut t = tera::Tera::default();
        t.add_raw_template("dockerfile", BASE_DOCKER_SRC)
            .expect("Failed to register docker source template");
        t.add_raw_template("install.sh", INSTALL_SRC)
            .expect("Failed to register install source template");
        t.add_raw_template("entrypoint.sh", ENTRY_SRC)
            .expect("Failed to register init source template");
        t
    };
}

pub async fn build_with_output(
    output: &Option<PathBuf>,
    config_file: &Path,
    tag: Option<String>,
) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::open(config_file)
        .await
        .context("Failed to open config file")?;
    let mut config_content = String::new();
    file.read_to_string(&mut config_content)
        .await
        .context("Failed to read config file to string")?;
    let cfg = bedrock::Config::from_str(
        &config_content,
        Some(config_file.to_str().context("Failed to get file path")?),
    )
    .context("Failed to read configuration file")?;

    let mut tarball = tokio_tar::Builder::new(Vec::new());

    let mut ctx = tera::Context::new();
    ctx.insert(
        "server_tag",
        &std::env::var("BASALT_SERVER_TAG").unwrap_or_else(|_| get_server_tag(&cfg)),
    );
    ctx.insert(
        "web_tag",
        &std::env::var("BASALT_WEB_TAG").unwrap_or(APP_VERSION.to_owned()),
    );
    ctx.insert("base_install", &make_base_install(&cfg));
    ctx.insert("base_init", &make_base_init(&cfg));
    if let Some(setup) = &cfg.setup {
        if let Some(install) = &setup.install {
            dbg!(install.to_string());
            ctx.insert("custom_install", &install.trim());
        }
        if let Some(init) = &setup.init {
            dbg!(init.to_string());
            ctx.insert("custom_init", init.trim());
        }
    }
    ctx.insert("web_client", &cfg.web_client);

    ctx.insert("event_handler_scripts", &cfg.integrations.event_handlers);

    let install_content = tmpl
        .render("install.sh", &ctx)
        .context("Failed to render installation script")?;

    let entrypoint_content = tmpl
        .render("entrypoint.sh", &ctx)
        .context("Failed to render entrypoint script")?;

    dbg!(&install_content);

    let content = tmpl
        .render("dockerfile", &ctx)
        .context("Failed to render dockerfile")?;

    let config_header = make_header("config.toml", config_content.len() as u64, 0o644)
        .context("Failed to create config header")?;
    tarball
        .append(&config_header, config_content.as_bytes())
        .await
        .context("Failed to archive config.toml")?;

    let dockerfile_header = make_header("Dockerfile", content.len() as u64, 0o644)
        .context("Failed to create dockerfile header")?;
    tarball
        .append(&dockerfile_header, content.as_bytes())
        .await
        .context("Failed to append dockerfile to tarball")?;

    let install_header = make_header("install.sh", install_content.len() as u64, 0o644)
        .context("Failed to create install header")?;
    tarball
        .append(&install_header, install_content.as_bytes())
        .await
        .context("Failed to append install.sh to tarball")?;

    let entrypoint_header = make_header("entrypoint.sh", entrypoint_content.len() as u64, 0o644)
        .context("Failed to create entrypoint header")?;
    tarball
        .append(&entrypoint_header, entrypoint_content.as_bytes())
        .await
        .context("Failed to append entrypoint.sh to tar")?;

    // add scripts if any exist
    append_event_handlers(&mut tarball, cfg.clone())
        .await
        .context("Failed to add scripts")?;

    let out_data = tarball
        .into_inner()
        .await
        .context("Failed to finish tarball")?;

    match output {
        Some(out_path) => {
            tokio::fs::write(out_path, out_data)
                .await
                .context("Failed to write data")?;
        }
        None => {
            let docker = bollard::Docker::connect_with_local_defaults()
                .context("Failed to connect to docker")?;
            let tag = tag.unwrap_or(format!("bslt-{}", cfg.hash()));
            let stream = docker.build_image(
                bollard::image::BuildImageOptions {
                    dockerfile: "Dockerfile",
                    t: &tag,
                    rm: true,
                    ..Default::default()
                },
                None,
                Some(out_data.into()),
            );

            // Process the stream
            tokio::pin!(stream);
            while let Some(item) = stream.next().await {
                let msg = item.context("Failed to perform docker build")?;
                if let Some(stream) = msg.stream {
                    println!(
                        "[BUILD] {}",
                        stream.trim().replace("\n", " ").replace("\t", " ")
                    );
                }
            }
        }
    };
    Ok(())
}

fn make_base_install(cfg: &Config) -> String {
    cfg.languages
        .iter()
        .map(|e| match e {
            bedrock::language::Language::BuiltIn { language, version } => {
                language.install_command(version).unwrap_or("").to_owned()
            }
            _ => "".into(),
        })
        .filter(|e| !e.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn make_base_init(cfg: &Config) -> String {
    cfg.languages
        .iter()
        .map(|e| match e {
            bedrock::language::Language::BuiltIn { language, version } => {
                language.init_command(version).unwrap_or("").to_owned()
            }
            _ => "".into(),
        })
        .filter(|e| !e.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn make_header<P>(path: P, size: u64, mode: u32) -> anyhow::Result<Header>
where
    P: AsRef<Path>,
{
    let mut header = tokio_tar::Header::new_gnu();
    header
        .set_path(&path)
        .with_context(|| format!("Failed to set {} tar header", path.as_ref().display()))?;
    header.set_size(size);
    header.set_mode(mode);
    header.set_cksum();
    Ok(header)
}

/// Based on the config, determine which tag to use
fn get_server_tag(cfg: &Config) -> String {
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

async fn append_event_handlers(tb: &mut Builder<Vec<u8>>, cfg: Config) -> anyhow::Result<()> {
    let mut set = JoinSet::new();

    for handler_path in cfg.integrations.event_handlers {
        set.spawn(async move {
            let contents = tokio::fs::read_to_string(&handler_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to read script contents from {}",
                        handler_path.display()
                    )
                })?;

            Ok::<_, anyhow::Error>((handler_path, contents))
        });
    }

    // Collect results (unordered)
    let results = set
        .join_all()
        .await
        .into_iter()
        .collect::<Result<Vec<(PathBuf, String)>, _>>()
        .context("Failed to read scripts")?;

    // Append sequentially (tarball writes must be ordered)
    for (handler_path, contents) in results {
        let script_header = make_header(&handler_path, contents.len() as u64, 0o644)
            .context("Failed to create script header")?;

        tb.append(&script_header, contents.as_bytes())
            .await
            .context("Failed to append script to tarball")?;
    }

    Ok(())
}
