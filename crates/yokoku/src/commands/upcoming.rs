use std::io;

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{app::App, commands::calendar::print_calendar, config::Config};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UpcomingConfig {
    pub days: u16,
}

impl Default for UpcomingConfig {
    fn default() -> Self {
        Self { days: 14 }
    }
}

#[derive(Parser)]
pub struct Args {
    /// Days ahead to include, overriding `upcoming.days`
    #[arg(long, short = 'd')]
    pub days: Option<u16>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &UpcomingConfig) -> UpcomingConfig {
        let mut resolved = config.clone();

        if let Some(days) = self.days {
            resolved.days = days;
        }

        resolved
    }
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let settings = args.apply_overrides(&config.upcoming);
    debug!(?settings, "resolved command settings");

    let app = App::open(config).await?;
    print_calendar(&mut io::stdout(), &app.schedule.upcoming(settings.days).await?)?;

    Ok(())
}
