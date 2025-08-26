use std::path::{Path, PathBuf};

use anyhow::Context;
use bedrock::Config;
use tokio::task::JoinSet;
use tokio_tar::{Builder, Header};

pub fn make_base_install(cfg: &Config) -> String {
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

pub fn make_base_init(cfg: &Config) -> String {
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

pub fn make_header<P>(path: P, size: u64, mode: u32) -> anyhow::Result<Header>
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

pub async fn append_event_handlers(tb: &mut Builder<Vec<u8>>, cfg: Config) -> anyhow::Result<()> {
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
