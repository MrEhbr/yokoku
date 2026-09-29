use std::io;

use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::info;
use yokoku_jobs::Jobs;

use crate::app::App;

/// Serves the web UI, delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(app: &App) -> Result<()> {
    let config = app.settings.current();
    let schedules = config.serve.schedules()?;
    let state = yokoku_web::AppState {
        library: app.library.clone(),
        artworks: app.artworks.clone(),
        calendar: app.calendar.clone(),
        prober: app.prober.clone(),
        history: app.history.clone(),
        clock: app.clock.clone(),
    };
    let web = yokoku_web::Server::bind(config.web.address(), state).await.context("Failed to start the web server")?;

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
    let web = tokio::spawn(web.serve(shutdown.clone().cancelled_owned()));
    info!("running");
    let result = monitor.run_with_signal(stop_signal()).await;

    shutdown.cancel();
    for delivery in deliveries {
        delivery.await.context("Event delivery failed")?;
    }
    web.await.context("Web server failed")?.context("Web server failed")?;
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
