use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use bedrock::language::Language;
use lazy_static::lazy_static;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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
    let cfg = bedrock::Config::read_async(
        &mut file,
        config_file.file_name().map(|s| s.to_string_lossy()),
    )
    .await
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
        .context("Failed to render installation script")?
        .replace("\n", DOCKER_SEP);
    let init_content = tmpl
        .render("init.sh", &ctx)
        .context("Failed to render init script")?
        .replace("\n", DOCKER_SEP);
    dbg!(&install_content);
    ctx.insert("installsh", &install_content);
    ctx.insert("initsh", &init_content);
    ctx.insert("entrypointsh", &ENTRY_SRC.replace("\n", DOCKER_SEP));
    let content = tmpl
        .render("dockerfile", &ctx)
        .context("Failed to render dockerfile")?;
    let mut install_header = tokio_tar::Header::new_gnu();
    install_header
        .set_path("install.sh")
        .context("Failed to set install.sh tar header")?;
    let mut entrypoint_header = tokio_tar::Header::new_gnu();
    entrypoint_header
        .set_path("entrypoint.sh")
        .context("Failed to set entrypoint.sh tar header")?;
    tarball
        .append(&entrypoint_header, entrypoint_content.as_bytes())
        .await
        .context("Failed to append install.sh to zip")?;
    if output.is_none() {
        // run docker build
    }
    Ok(())
}
