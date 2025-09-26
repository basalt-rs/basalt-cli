use std::{collections::HashMap, net::Ipv4Addr, process::ExitStatus};

use anyhow::{bail, Context};
use dialoguer::{theme::ColorfulTheme, Input, Password};
use tokio::{io, process::Command};

use crate::utils::code_to_address;

pub async fn handle(
    code: Option<String>,
    addr: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> anyhow::Result<()> {
    let host: String = if let Some(host) = addr {
        format!("http://{}", host)
    } else if let Some(code) = code {
        // convert game code to IP with port
        let (ip, port) = code_to_address(&code).context("Invalid gamecode")?;
        format!("http://{}:{}", ip, port)
    } else {
        bail!("Must provide code or addr")
    };

    let username = username
        .context("Username not provided in CLI")
        .or(Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter username")
            .interact_text()
            .context("Failed to obtain username via input from user"))
        .context("Failed to determine username")?;

    let password = password
        .context("Password not provided in CLI")
        .or(Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter password")
            .with_confirmation("Confirm password", "Passwords do not match")
            .interact()
            .context("Failed to obtain password via input from user"))
        .context("Failed to determine password")?;

    let mut body_map = HashMap::new();
    body_map.insert("username", username);
    body_map.insert("password", password);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/login", host))
        .json(&body_map)
        .send()
        .await
        .context("Failed to log in")?;

    let session_data = response
        .json::<HashMap<String, String>>()
        .await
        .context("Failed to parse response")?;

    let env_overrides = [(
        "BASALT_SESSION_TOKEN",
        session_data
            .get("token")
            .context("Response missing token")?,
    )];

    let status = spawn_subshell(env_overrides).await?;
    Ok(())
}

pub async fn spawn_subshell<I, K, V>(overrides: I) -> io::Result<ExitStatus>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let (path, args) = default_shell_command();

    let mut cmd = Command::new(path);
    cmd.args(args);

    // Use the real TTY (so it acts just like their shell)
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    cmd.envs(overrides.into_iter().map(|(k, v)| (k.into(), v.into())));

    // If you want to wait async while still responding to signals:
    let mut child = cmd.spawn()?;

    // Wait for the shell to exit
    let status = child.wait().await?;
    Ok(status)
}

/// Detect default shell + args
#[cfg(unix)]
fn default_shell_command() -> (String, Vec<String>) {
    use std::{env, path::Path};

    let shell_path = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let name = Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    let args: Vec<String> = match name {
        "bash" => vec!["-l".into(), "-i".into()],
        "zsh" => vec!["-l".into(), "-i".into()],
        "fish" => vec!["-l".into(), "-i".into()],
        "sh" | "dash" => vec!["-i".into()],
        _ => vec!["-i".into()],
    };

    (shell_path, args)
}

#[cfg(windows)]
fn default_shell_command() -> (String, Vec<String>) {
    let comspec =
        env::var("COMSPEC").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());
    (comspec, vec![])
}
