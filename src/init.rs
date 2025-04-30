use std::path::PathBuf;

use anyhow::Context;
use lazy_static::lazy_static;

const BASE_DEFAULT_CONFIG: &str = include_str!("../data/default.toml");

lazy_static! {
    static ref tmpl: tera::Tera = {
        let mut t = tera::Tera::default();
        t.add_raw_template("template", BASE_DEFAULT_CONFIG)
            .expect("Failed to register docker source template");
        t
    };
}

pub async fn handle(path: Option<PathBuf>) -> anyhow::Result<()> {
    // configuration name should equal the filename in the path provided or default to `basalt` if no path is provided or the filename is empty.
    let name = if let Some(mut path) = path.clone() {
        path.set_extension("");
        path.file_name()
            .map(|f| f.to_str())
            .flatten()
            .map_or("basalt", |f| {
                if f.trim().is_empty() {
                    "basalt"
                } else {
                    f.trim()
                }
            })
            .to_owned()
    } else {
        "basalt".to_owned()
    };

    let path = path.map_or(PathBuf::from(&name), |p| {
        if p.file_name().is_some() {
            p
        } else {
            p.with_file_name(name.clone()).with_extension("toml")
        }
    });

    let mut ctx = tera::Context::new();
    ctx.insert("name", &name);
    let content = tmpl
        .render("template", &ctx)
        .context("Failed to render template")?;

    tokio::fs::write(path, content)
        .await
        .context("Failed to write data")?;

    Ok(())
}
