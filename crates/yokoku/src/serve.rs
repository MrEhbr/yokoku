use std::io;

use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{app::App, background::Background};

const ROOTS_INVALID: &str = concat!(
    "The root folders in the config file are invalid; fix them, or remove the stored root folder they ",
    "conflict with using `yokoku root remove`",
);

/// Serves the web UI, delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(app: &App, background: &Background) -> Result<()> {
    app.roots.check().await.context(ROOTS_INVALID)?;
    let (stop_jobs, stop_service) = (CancellationToken::new(), CancellationToken::new());
    let web = yokoku_web::Server::bind(app.settings.current().web.address(), app.into())
        .await
        .context("Failed to start the web server")?;

    let jobs = background.scheduler().spawn(&stop_jobs);
    let deliveries = background.spawn_deliveries(&stop_service);
    let web = tokio::spawn(web.serve(stop_service.clone()));
    info!("running");
    let signal = stop_signal().await;

    stop_jobs.cancel();
    jobs.join_all().await;
    stop_service.cancel();
    deliveries.join_all().await;
    web.await.context("Web server failed")??;
    if let Err(error) = background.flush().await {
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
