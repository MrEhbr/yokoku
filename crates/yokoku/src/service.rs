use std::{io, sync::Arc};

use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::app::App;

/// Serves the web UI, delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(app: &App) -> Result<()> {
    app.roots.check().await.context(
        "The root folders in the config file are invalid; fix them, or remove the stored root folder they \
         conflict with using `yokoku root remove`",
    )?;
    let (stop_jobs, stop_service) = (CancellationToken::new(), CancellationToken::new());
    let state = yokoku_web::AppState {
        library: app.library.clone(),
        artworks: app.artworks.clone(),
        calendar: app.calendar.clone(),
        prober: app.prober.clone(),
        history: app.history.clone(),
        clock: app.clock.clone(),
        downloads: app.downloads.clone(),
        releases: app.releases.clone(),
        reviewer: app.reviewer.clone(),
        importer: app.importer.clone(),
        queue_changes: Arc::new(app.queue_changes.clone()),
        metadata: app.metadata.clone(),
        roots: app.roots.clone(),
        deleter: app.deleter.clone(),
        renamer: app.renamer.clone(),
        scanner: app.scanner.clone(),
        settings: Arc::new(app.settings.clone()),
        connections: Arc::new(app.settings.clone()),
        add: Arc::new(yokoku_web::AddSettings {
            tmdb_token_set: app.settings.live(|config| config.metadata.tmdb.token.is_some()),
            monitor: app.settings.live(|config| config.add.monitor),
        }),
        shutdown: Arc::new(stop_service.clone()),
    };
    let web = yokoku_web::Server::bind(app.settings.current().web.address(), state)
        .await
        .context("Failed to start the web server")?;

    let jobs = crate::jobs::spawn(app, &stop_jobs);
    let deliveries = app.spawn_deliveries(&stop_service);
    let web = tokio::spawn(web.serve(stop_service.clone().cancelled_owned()));
    info!("running");
    let signal = stop_signal().await;

    stop_jobs.cancel();
    jobs.join_all().await;
    stop_service.cancel();
    deliveries.join_all().await;
    web.await.context("Web server failed")??;
    if let Err(error) = app.events.flush().await {
        error!(%error, "events of saved changes were lost; their handlers will not run");
    }
    info!("stopped");
    signal.context("Failed to wait for a stop signal")
}

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
