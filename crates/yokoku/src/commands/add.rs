use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tracing::debug;
use yokoku_domain::MonitorPreset;

use crate::{
    app::App,
    commands::{ItemArgs, Kind, title_with_year},
    config::Config,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct AddConfig {
    pub monitor: Monitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Monitor {
    /// Every episode, or the movie
    #[default]
    All,
    /// Episodes that have not aired yet
    Future,
    /// Episodes of the latest season
    LatestSeason,
    /// Nothing
    None,
}

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,

    /// What to monitor, overriding `add.monitor`; movies accept `all` or `none`
    #[arg(long)]
    pub monitor: Option<Monitor>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &AddConfig) -> AddConfig {
        let mut resolved = config.clone();

        if let Some(monitor) = self.monitor {
            resolved.monitor = monitor;
        }

        resolved
    }
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let settings = args.apply_overrides(&config.add);
    debug!(?settings, "resolved command settings");

    let app = App::open(config).await?;
    let sync = app.sync()?;
    match args.item.kind {
        Kind::Series => {
            let series = sync.add_series(args.item.source, settings.monitor.into()).await?;
            println!("Added series {} {}", title_with_year(&series.title, series.year), series.source);
        },
        Kind::Movie => {
            let monitored = match settings.monitor {
                Monitor::All => true,
                Monitor::None => false,
                Monitor::Future | Monitor::LatestSeason => bail!("movies can only be monitored with `all` or `none`"),
            };
            let movie = sync.add_movie(args.item.source, monitored).await?;
            println!("Added movie {} {}", title_with_year(&movie.title, movie.year), movie.source);
        },
    }

    Ok(())
}

impl From<Monitor> for MonitorPreset {
    fn from(monitor: Monitor) -> Self {
        match monitor {
            Monitor::All => Self::All,
            Monitor::Future => Self::Future,
            Monitor::LatestSeason => Self::LatestSeason,
            Monitor::None => Self::None,
        }
    }
}
