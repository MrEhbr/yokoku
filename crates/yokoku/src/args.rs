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
}

impl Args {
    fn resolve_config(&self) -> Result<Config> {
        let mut config: Config = crate::config::load(self.config.as_deref()).context("Failed to load configuration")?;

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

    let config = args.resolve_config()?;
    let _guard = logging::setup(&config.log).context("Failed to initialize logging")?;

    match args.command {
        Greet(cmd_args) => commands::greet::run(&config, cmd_args),
        List(cmd_args) => commands::list::run(&config, cmd_args).await,
        Show(cmd_args) => commands::show::run(&config, cmd_args).await,
        Monitor(cmd_args) => commands::monitor::run(&config, cmd_args).await,
        Numbering(cmd_args) => commands::numbering::run(&config, cmd_args).await,
        Remove(cmd_args) => commands::remove::run(&config, cmd_args).await,
    }
}
