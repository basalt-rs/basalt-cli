use containers::{build_container_image, get_server_tag};
use std::path::{Path, PathBuf};
use tar_helpers::{append_event_handlers, make_base_init, make_base_install, make_header};

use anyhow::Context;
use lazy_static::lazy_static;
use tokio::io::AsyncReadExt;

use crate::cli::ContainerBackend;

mod containers;
mod tar_helpers;

const BASE_DOCKER_SRC: &str = include_str!("../../data/basalt.Dockerfile");
const INSTALL_SRC: &str = include_str!("../../data/install.sh");
const ENTRY_SRC: &str = include_str!("../../data/entrypoint.sh");
const DOCKER_IGNORE: &str = "./Dockerfile\n./.dockerignore";

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
    container_backend: ContainerBackend,
    verbose: bool,
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

    let docker_ignore_header = make_header(".dockerignore", DOCKER_IGNORE.len() as u64, 0o644)
        .context("Failed to create dockerignore header")?;
    tarball
        .append(&docker_ignore_header, DOCKER_IGNORE.as_bytes())
        .await
        .context("Failed to append .dockerignore to tarball")?;

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
        None => build_container_image(
            out_data,
            tag.unwrap_or_else(|| format!("bslt-{}", cfg.hash())),
            container_backend,
            verbose,
        )
        .await
        .context("Failed to build container image")?,
    };
    Ok(())
}
