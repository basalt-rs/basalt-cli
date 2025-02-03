use std::path::Path;

use anyhow::{bail, Context};
use bedrock::language::{BuiltInLanguage, Language};
use lazy_static::lazy_static;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const BASE_DOCKER_SRC: &str = include_str!("../data/basalt.Dockerfile");
const DOCKER_SEP: &str = "\\\n";
const INSTALL_SRC: &str = include_str!("../data/install.sh");

lazy_static! {
    static ref tmpl: tera::Tera = {
        let mut t = tera::Tera::default();
        t.add_raw_template("dockerfile", BASE_DOCKER_SRC)
            .expect("Failed to register docker source template");
        t.add_raw_template("install.sh", INSTALL_SRC)
            .expect("Failed to register install source template");
        t
    };
}

pub async fn build(output: &Path, config_file: &Path) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::open(config_file)
        .await
        .context("Failed to open config file")?;
    let cfg = bedrock::Config::read_async(
        &mut file,
        config_file.file_name().map(|s| s.to_string_lossy()),
    )
    .await
    .context("Failed to read configuration file")?;

    let mut ctx = tera::Context::new();
    dbg!(cfg.languages);
    if let Some(setup) = &cfg.setup {
        if let Some(install) = &setup.install {
            dbg!(install.to_string());
            ctx.insert("custom_install", &install.trim());
            ctx.insert("base_install", "dnf install python3");
            let install_content = tmpl
                .render("install.sh", &ctx)
                .context("Failed to render installation script")?;
            dbg!(&install_content);
            ctx.insert("install.sh", &install_content);
        }
        if let Some(init) = &setup.init {
            dbg!(init.to_string());
            ctx.insert("init", init.trim());
        }
    }
    let content = tmpl
        .render("dockerfile", &ctx)
        .context("Failed to render dockerfile")?;
    let mut outfile = File::create(output)
        .await
        .context("Failed to open specified output file")?;
    outfile
        .write(content.as_bytes())
        .await
        .context("Failed to write rendered Dockerfile")?;
    bail!("Unimplemented")
}

fn install(lang: Language) {}
