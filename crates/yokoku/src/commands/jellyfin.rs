use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{app::App, config::Config, secret::Secret};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct JellyfinConfig {
    /// Server address, e.g. `http://localhost:8096`; rescans are off while unset.
    pub url: Option<String>,
    /// An administrator's API key.
    pub api_key: Option<Secret>,
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
    match args.command {
        Command::Test => success!("Connected to {}", rescans.test_connection().await?)?,
        Command::Rescan => {
            rescans.run_now().await?;
            say!("Jellyfin is rescanning")?;
        },
    }
    Ok(())
}
