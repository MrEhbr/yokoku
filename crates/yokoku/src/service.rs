use std::io;

use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::info;
use yokoku_jobs::Jobs;

use crate::app::App;

/// Delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(app: &App) -> Result<()> {
    let config = app.settings.current();
    let schedules = config.serve.schedules()?;

    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let monitor = yokoku_jobs::monitor(
        Jobs {
            downloads: app.downloads.clone(),
            importer: app.importer.clone(),
            scanner: app.scanner.clone(),
            metadata: app.metadata_service(),
            rescans: app.rescans.clone(),
        },
        schedules,
    );
    info!("running");
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
