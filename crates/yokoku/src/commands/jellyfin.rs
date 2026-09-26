use std::{
    fmt,
    io::{self, Write},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{app::App, config::Config};

#[derive(Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct JellyfinConfig {
    /// Server address, e.g. `http://localhost:8096`; rescans are off while unset.
    pub url: Option<String>,
    /// An administrator's API key; set it through `APP__JELLYFIN__API_KEY`.
    pub api_key: Option<String>,
}

impl fmt::Debug for JellyfinConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JellyfinConfig")
            .field("url", &self.url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check that Jellyfin can be reached with the API key
    Test,
    /// Ask Jellyfin to rescan its libraries now
    Rescan,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let rescans =
        app.rescans.as_ref().context("No Jellyfin configured; set jellyfin.url and APP__JELLYFIN__API_KEY")?;
    let mut out = io::stdout();
    match args.command {
        Command::Test => writeln!(out, "Connected to {}", rescans.test_connection().await?)?,
        Command::Rescan => {
            rescans.run_now().await?;
            writeln!(out, "Jellyfin is rescanning")?;
        },
    }
    Ok(())
}
