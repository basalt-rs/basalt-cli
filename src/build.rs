use std::path::{Path, PathBuf};

use anyhow::Context;
use bedrock::Config;
use futures::StreamExt;
use lazy_static::lazy_static;
use tokio::io::AsyncReadExt;
use tokio_tar::Header;

const BASE_DOCKER_SRC: &str = include_str!("../data/basalt.Dockerfile");
const INSTALL_SRC: &str = include_str!("../data/install.sh");
const ENTRY_SRC: &str = include_str!("../data/entrypoint.sh");

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
