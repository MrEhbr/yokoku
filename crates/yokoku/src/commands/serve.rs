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
    /// Cron schedule with seconds for checking whether Jellyfin should rescan.
    pub rescan_media_server: String,
    /// Cron schedule with seconds for refreshing every item from TMDB.
    pub refresh_metadata: String,
    /// Cron schedule with seconds for scanning root folders for outside changes.
    pub scan_library: String,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            sync_downloads: "*/30 * * * * *".into(),
            execute_imports: "*/5 * * * * *".into(),
            rescan_media_server: "*/10 * * * * *".into(),
            refresh_metadata: "0 0 */6 * * *".into(),
            scan_library: "0 0 5 * * *".into(),
        }
    }
}

impl ServeConfig {
    pub fn schedules(&self) -> Result<Schedules> {
        Ok(Schedules {
            sync_downloads: Self::schedule(&self.sync_downloads)?,
            execute_imports: Self::schedule(&self.execute_imports)?,
            rescan_media_server: Self::schedule(&self.rescan_media_server)?,
            refresh_metadata: Self::schedule(&self.refresh_metadata)?,
            scan_library: Self::schedule(&self.scan_library)?,
        })
    }

    fn schedule(expression: &str) -> Result<Schedule> {
        Schedule::from_str(expression).with_context(|| format!("Invalid schedule: {expression}"))
    }
}

#[derive(Parser)]
pub struct Args {}

/// Delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let schedules = config.serve.schedules()?;
    let app = App::open(config).await?;

    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let monitor = yokoku_jobs::monitor(
        Jobs {
            downloads: app.downloads.clone(),
            importer: app.importer.clone(),
            scanner: app.scanner.clone(),
            metadata: app.metadata(),
            rescans: app.rescans.clone(),
        },
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

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
