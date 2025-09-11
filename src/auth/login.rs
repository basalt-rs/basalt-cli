use std::net::Ipv4Addr;

use anyhow::Context;
use dialoguer::{theme::ColorfulTheme, Input, Password};

pub async fn handle(
    code: Option<String>,
    host: Option<Ipv4Addr>,
    username: Option<String>,
    password: Option<String>,
) -> anyhow::Result<()> {
    let host: Ipv4Addr = if let Some(host) = host {
        host
    } else if let Some(code) = code {
        // convert game code to IP with port
        todo!()
    } else {
        // Simply assuming IP for now
        Input::<Ipv4Addr>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter host IP or gamecode")
            .interact_text()
            .context("Failed to obtain host from user")?
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

    dbg!(
        "Read data: host={}, username={}, password={}",
        &host,
        &username,
        &password
    );

    todo!()
}
