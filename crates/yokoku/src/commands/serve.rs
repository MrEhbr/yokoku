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
    /// Cron schedule with seconds for carrying out approved imports.
    pub execute_imports: String,
    /// Cron schedule with seconds for removing old recycled files.
    pub cleanup_recycle: String,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            sync_downloads: "*/30 * * * * *".into(),
            execute_imports: "*/5 * * * * *".into(),
            cleanup_recycle: "0 0 4 * * *".into(),
        }
    }
}

#[derive(Parser)]
pub struct Args {}

/// Delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let schedules = Schedules {
        sync_downloads: schedule(&config.serve.sync_downloads)?,
        execute_imports: schedule(&config.serve.execute_imports)?,
        cleanup_recycle: schedule(&config.serve.cleanup_recycle)?,
    };
    let app = App::open(config).await?;
    let recovered = app.importer.recover().await?;
    if recovered > 0 {
        info!(recovered, "queued interrupted imports again");
    }

    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let monitor = yokoku_jobs::monitor(
        Jobs { downloads: app.downloads.clone(), importer: app.importer.clone(), deleter: app.deleter.clone() },
        schedules,
    );
    info!("serving");
    let result = monitor.run_with_signal(stop_signal()).await;

    shutdown.cancel();
    for delivery in deliveries {
        delivery.await.context("Event delivery failed")?;
    }
    info!("stopped");
    result.context("Jobs failed")
}

fn schedule(expression: &str) -> Result<Schedule> {
    Schedule::from_str(expression).with_context(|| format!("Invalid schedule: {expression}"))
}

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
