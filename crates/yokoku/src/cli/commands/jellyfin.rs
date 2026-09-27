use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use crate::app::App;

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

pub async fn run(app: &App, args: Args) -> Result<()> {
    if app.settings.current().jellyfin.url.is_none() {
        bail!("No Jellyfin configured; set jellyfin.url and APP__JELLYFIN__API_KEY");
    }
    let rescans = &app.rescans;
    match args.command {
        Command::Test => success!("Connected to {}", rescans.test_connection().await?)?,
        Command::Rescan => {
            rescans.run_now().await?;
            say!("Jellyfin is rescanning")?;
        },
    }
    Ok(())
}
