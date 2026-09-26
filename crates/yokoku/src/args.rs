use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

use crate::{commands, config::Config, logging};

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, short = 'c', value_name = "FILE", global = true)]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Greet someone
    Greet(commands::greet::Args),
    /// Search the metadata source for series and movies
    Search(commands::search::Args),
    /// Add a series or movie to the library
    Add(commands::add::Args),
    /// Refresh metadata for one item or the whole library
    Refresh(commands::refresh::Args),
    /// List library items
    List(commands::list::Args),
    /// Show a series or movie in detail
    Show(commands::show::Args),
    /// Mark a series, season, episode or movie monitored or not
    Monitor(commands::monitor::Args),
    /// Set a series' episode numbering
    Numbering(commands::numbering::Args),
    /// Remove a series or movie from the library
    Remove(commands::remove::Args),
    /// List monitored releases coming up
    Upcoming(commands::upcoming::Args),
    /// Show monitored releases for a week or month
    Calendar(commands::calendar::Args),
    /// List monitored episodes and movies that are out but have no file
    Missing(commands::missing::Args),
    /// Manage the folders that hold series and movies
    Root(commands::root::Args),
    /// Link files in the root folders to the library
    Scan(commands::scan::Args),
    /// Match files the scan was unsure about
    Review(commands::review::Args),
    /// Move library files to the names and folders naming gives them
    Rename(commands::rename::Args),
    /// Add torrents to Transmission and follow their progress
    Download(commands::download::Args),
    /// Follow and retry imports of finished downloads
    Import(commands::import::Args),
    /// Show what happened, newest first
    History(commands::history::Args),
    /// Delete the file of an episode or movie, or move it to the recycle folder
    Delete(commands::delete::Args),
    /// Show the details of library files, or read them with ffprobe
    Files(commands::files::Args),
    /// Manage the recycle folder
    Recycle(commands::recycle::Args),
    /// Test the Jellyfin connection or ask it to rescan
    Jellyfin(commands::jellyfin::Args),
    /// Store settings in the database, over the config file
    Settings(commands::settings::Args),
    /// Deliver events and run scheduled jobs until stopped
    Serve(commands::serve::Args),
}

impl Args {
    async fn resolve_config(&self) -> Result<Config> {
        let path = self.config.as_deref();
        let mut config: Config = crate::config::load(path, &[]).context("Failed to load configuration")?;
        if !matches!(self.command, Command::Settings(_)) {
            let stored = commands::settings::stored(&config.database.path).await?;
            if !stored.is_empty() {
                config = crate::config::load(path, &stored)
                    .context("Failed to load configuration with the stored settings; see `yokoku settings list`")?;
            }
        }

        // `tracing_level()` yields the default level even when no flag was
        // passed, so only consult it when the user actually supplied one.
        if self.verbosity.is_present() {
            config.log.level = self.verbosity.tracing_level();
        }

        Ok(config)
    }
}

pub async fn route(args: Args) -> Result<()> {
    use Command::*;

    let config = args.resolve_config().await?;
    let _guard = logging::setup(&config.log).context("Failed to initialize logging")?;

    match args.command {
        Greet(cmd_args) => commands::greet::run(&config, cmd_args),
        Search(cmd_args) => commands::search::run(&config, cmd_args).await,
        Add(cmd_args) => commands::add::run(&config, cmd_args).await,
        Refresh(cmd_args) => commands::refresh::run(&config, cmd_args).await,
        List(cmd_args) => commands::list::run(&config, cmd_args).await,
        Show(cmd_args) => commands::show::run(&config, cmd_args).await,
        Monitor(cmd_args) => commands::monitor::run(&config, cmd_args).await,
        Numbering(cmd_args) => commands::numbering::run(&config, cmd_args).await,
        Remove(cmd_args) => commands::remove::run(&config, cmd_args).await,
        Upcoming(cmd_args) => commands::upcoming::run(&config, cmd_args).await,
        Calendar(cmd_args) => commands::calendar::run(&config, cmd_args).await,
        Missing(cmd_args) => commands::missing::run(&config, cmd_args).await,
        Root(cmd_args) => commands::root::run(&config, cmd_args).await,
        Scan(cmd_args) => commands::scan::run(&config, cmd_args).await,
        Review(cmd_args) => commands::review::run(&config, cmd_args).await,
        Rename(cmd_args) => commands::rename::run(&config, cmd_args).await,
        Download(cmd_args) => commands::download::run(&config, cmd_args).await,
        Import(cmd_args) => commands::import::run(&config, cmd_args).await,
        History(cmd_args) => commands::history::run(&config, cmd_args).await,
        Delete(cmd_args) => commands::delete::run(&config, cmd_args).await,
        Files(cmd_args) => commands::files::run(&config, cmd_args).await,
        Recycle(cmd_args) => commands::recycle::run(&config, cmd_args).await,
        Jellyfin(cmd_args) => commands::jellyfin::run(&config, cmd_args).await,
        Settings(cmd_args) => commands::settings::run(&config, args.config.as_deref(), cmd_args).await,
        Serve(cmd_args) => commands::serve::run(&config, cmd_args).await,
    }
}
