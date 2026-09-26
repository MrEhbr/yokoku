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

pub fn route(args: Args) -> Result<()> {
    use Command::*;

    let config = args.resolve_config()?;
    let _guard = logging::setup(&config.log).context("Failed to initialize logging")?;

    match args.command {
        Greet(cmd_args) => commands::greet::run(&config, cmd_args),
    }
}
