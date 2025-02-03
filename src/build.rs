use std::path::Path;

use anyhow::{bail, Context};
use lazy_static::lazy_static;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const DOCKER_SRC: &str = include_str!("../data/basalt.Dockerfile");

lazy_static! {
    static ref tmpl: tera::Tera = {
        let mut t = tera::Tera::default();
        t.add_raw_template("dockerfile", DOCKER_SRC)
            .expect("Failed to register template");
        t
    };
}

pub async fn build(output: &Path, config_file: &Path) -> anyhow::Result<()> {
    let mut ctx = tera::Context::new();
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
