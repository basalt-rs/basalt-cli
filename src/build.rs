use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use bedrock::language::Language;
use lazy_static::lazy_static;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BASE_DOCKER_SRC: &str = include_str!("../data/basalt.Dockerfile");
const DOCKER_SEP: &str = "\\\n";
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

    let (outfile, tf) = match output {
        Some(path) => (
            File::create(path).await.context("Failed to create file")?,
            None,
        ),
        None => {
            let tempfile = async_tempfile::TempFile::new()
                .await
                .context("Failed to create tempfile")?;

            let tempfile_clone = tempfile
                .try_clone()
                .await
                .context("Failed to clone tempdir")?;

            (
                File::create(&tempfile_clone.file_path())
                    .await
                    .context("Failed to create writable tempfile")?,
                Some(tempfile),
            )
        }
    };
    dbg!(&tf);
    let mut tarball = tokio_tar::Builder::new(Box::new(outfile));

    let mut ctx = tera::Context::new();
    ctx.insert("base_install", "dnf install python3");
    ctx.insert("base_init", "opam init -y\neval $(opam env)");
    dbg!(cfg.languages);
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
    let mut config_header = tokio_tar::Header::new_gnu();
    config_header
        .set_path("config.toml")
        .context("Failed to set config.toml header")?;
    config_header.set_size(config_content.len() as u64);
    config_header.set_mode(08_644);
    config_header.set_cksum();
    tarball
        .append(&config_header, config_content.as_bytes())
        .await
        .context("Failed to archive config.toml")?;
    let mut dockerfile_header = tokio_tar::Header::new_gnu();
    dockerfile_header
        .set_path("Dockerfile")
        .context("Failed to set Dockerfile tar header")?;
    dockerfile_header.set_size(content.len() as u64);
    dockerfile_header.set_mode(08_644);
    dockerfile_header.set_cksum();
    tarball
        .append(&dockerfile_header, content.as_bytes())
        .await
        .context("Failed to append dockerfile to tarball")?;
    let mut install_header = tokio_tar::Header::new_gnu();
    install_header
        .set_path("install.sh")
        .context("Failed to set install.sh tar header")?;
    install_header.set_size(install_content.len() as u64);
    install_header.set_mode(08_644);
    install_header.set_cksum();
    tarball
        .append(&install_header, install_content.as_bytes())
        .await
        .context("Failed to append install.sh to tarball")?;
    let mut entrypoint_header = tokio_tar::Header::new_gnu();
    entrypoint_header
        .set_path("entrypoint.sh")
        .context("Failed to set entrypoint.sh tar header")?;
    entrypoint_header.set_size(entrypoint_content.len() as u64);
    entrypoint_header.set_mode(08_644);
    entrypoint_header.set_cksum();
    tarball
        .append(&entrypoint_header, entrypoint_content.as_bytes())
        .await
        .context("Failed to append entrypoint.sh to tar")?;
    if output.is_none() {
        // run docker build
        bail!("Dockerfile publish not implemented");
    }
    Ok(())
}
