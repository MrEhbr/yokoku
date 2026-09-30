use std::{io, sync::Arc};

use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    app::{App, Connections},
    jobs::Jobs,
};

/// Serves the web UI, delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(app: &App) -> Result<()> {
    let config = app.settings.current();
    let state = yokoku_web::AppState {
        library: app.library.clone(),
        artworks: app.artworks.clone(),
        calendar: app.calendar.clone(),
        prober: app.prober.clone(),
        history: app.history.clone(),
        clock: app.clock.clone(),
        downloads: app.downloads.clone(),
        reviewer: app.reviewer.clone(),
        importer: app.importer.clone(),
        queue_changes: Arc::new(app.queue_changes.clone()),
        metadata: app.metadata_service(),
        roots: app.roots.clone(),
        deleter: app.deleter.clone(),
        renamer: app.renamer.clone(),
        scanner: app.scanner.clone(),
        settings: Arc::new(app.settings.clone()),
        connections: Arc::new(Connections(app.settings.clone())),
        add: Arc::new(yokoku_web::AddSettings {
            tmdb_token_set: app.settings.live(|config| config.metadata.tmdb.token.is_some()),
            monitor: app.settings.live(|config| config.add.monitor),
        }),
    };
    let web = yokoku_web::Server::bind(config.web.address(), state).await.context("Failed to start the web server")?;

    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let stop_jobs = shutdown.child_token();
    let jobs = crate::jobs::spawn(
        Jobs {
            downloads: app.downloads.clone(),
            importer: app.importer.clone(),
            scanner: app.scanner.clone(),
            metadata: app.metadata_service(),
            rescans: app.rescans.clone(),
        },
        app.settings.live(|config| config.serve.schedules()),
        &stop_jobs,
    );
    let web = tokio::spawn(web.serve(shutdown.clone().cancelled_owned()));
    info!("running");
    let result = stop_signal().await;

    stop_jobs.cancel();
    for job in jobs {
        job.await.context("Job failed")?;
    }
    shutdown.cancel();
    for delivery in deliveries {
        delivery.await.context("Event delivery failed")?;
    }
    web.await.context("Web server failed")?.context("Web server failed")?;
    if let Err(error) = app.events.flush().await {
        error!(%error, "events of saved changes were lost; their handlers will not run");
    }
    info!("stopped");
    result.context("Failed to wait for a stop signal")
}

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
