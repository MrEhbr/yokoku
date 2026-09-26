use std::{io, str::FromStr};

use anyhow::{Context, Result};
use clap::Parser;
use jiff_cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::info;
use yokoku_jobs::{Jobs, Schedules};

use crate::{app::App, config::Config};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ServeConfig {
    /// Cron schedule with seconds for syncing downloads.
    pub sync_downloads: String,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self { sync_downloads: "*/30 * * * * *".into() }
    }
}

#[derive(Parser)]
pub struct Args {}

/// Delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let schedule = &config.serve.sync_downloads;
    let schedules = Schedules {
        sync_downloads: Schedule::from_str(schedule).with_context(|| format!("Invalid schedule: {schedule}"))?,
    };
    let app = App::open(config).await?;

    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let monitor = yokoku_jobs::monitor(Jobs { downloads: app.downloads.clone() }, schedules);
    info!("serving");
    let result = monitor.run_with_signal(stop_signal()).await;

    shutdown.cancel();
    for delivery in deliveries {
        delivery.await.context("Event delivery failed")?;
    }
    info!("stopped");
    result.context("Jobs failed")
}

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
