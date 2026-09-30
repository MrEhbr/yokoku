use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use tracing::{Instrument, error, info_span};
use yokoku_config::{Config, LogOutput};
use yokoku_events::{CorrelationId, correlation::correlate};

use crate::{app::App, cli::commands, logging};

/// Attribution TMDB and TheTVDB require.
const DATA_SOURCES: &str = "This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise \
                            approved by TMDB (https://www.themoviedb.org).\nMetadata provided by TheTVDB \
                            (https://thetvdb.com). Please consider adding missing information or subscribing.";

#[derive(Parser)]
#[command(
    version,
    about,
    before_help = "Without a command, serves the web interface and runs jobs until stopped.",
    after_help = DATA_SOURCES
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, short = 'c', value_name = "FILE", global = true, default_value = "config/app.toml")]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Refresh metadata for one item or the whole library
    Refresh(commands::refresh::Args),
    /// Manage the folders that hold series and movies
    Root(commands::root::Args),
    /// Link files in the root folders to the library
    Scan(commands::scan::Args),
    /// Show the details of library files, or read them with ffprobe
    Files(commands::files::Args),
    /// Test the Jellyfin connection or ask it to rescan
    Jellyfin(commands::jellyfin::Args),
    /// Store settings in the database, over the config file
    Settings(commands::settings::Args),
}

impl Args {
    /// The configuration from the config file and the environment; the stored settings are read
    /// once the database opens.
    fn resolve_config(&self) -> Result<Config> {
        let mut config = Config::load(self.config.as_deref(), &[]).context("Failed to load configuration")?;

        // `tracing_level()` yields the default level even when no flag was
        // passed, so only consult it when the user actually supplied one.
        if self.verbosity.is_present() {
            config.log.level = self.verbosity.tracing_level();
        }

        Ok(config)
    }
}

/// Runs `command` in a `command` span under a new correlation id; a failure also goes to a log file.
pub async fn route(args: Args, command: &str) -> Result<()> {
    let config = args.resolve_config()?;
    let _guard = logging::setup(&config.log).context("Failed to initialize logging")?;

    let correlation = CorrelationId::generate();
    let span = info_span!("command", name = command, %correlation);
    let result = correlate(correlation, dispatch(&config, args).instrument(span.clone())).await;
    if let Err(failure) = &result
        && matches!(config.log.output, LogOutput::File(_))
    {
        span.in_scope(|| error!(error = format!("{failure:#}"), "command failed"));
    }
    result
}

async fn dispatch(config: &Config, args: Args) -> Result<()> {
    use Command::*;

    let command = match args.command {
        Some(Settings(cmd_args)) => return commands::settings::run(config, args.config.as_deref(), cmd_args).await,
        command => command,
    };
    let app = App::open(config, args.config.as_deref()).await?;
    let Some(command) = command else {
        return crate::service::run(&app).await;
    };
    let result = match command {
        Refresh(cmd_args) => commands::refresh::run(&app, cmd_args).await,
        Root(cmd_args) => commands::root::run(&app, cmd_args).await,
        Scan(cmd_args) => commands::scan::run(&app, cmd_args).await,
        Files(cmd_args) => commands::files::run(&app, cmd_args).await,
        Jellyfin(cmd_args) => commands::jellyfin::run(&app, cmd_args).await,
        Settings(_) => unreachable!("settings run before the app opens"),
    };
    result.and(app.events.flush().await.context(EVENTS_LOST))
}

pub const EVENTS_LOST: &str =
    "The change was saved, but its events could not be recorded, so the service will not react to it";
