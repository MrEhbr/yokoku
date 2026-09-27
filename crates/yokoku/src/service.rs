use std::{env, io, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use jiff_cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tokio_util::sync::CancellationToken;
use tracing::info;
use yokoku_jobs::{Jobs, Schedules};
use yokoku_web::Server;

use crate::{app::App, config::Config};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ServeConfig {
    /// Cron schedule with seconds for syncing downloads.
    pub sync_downloads: String,
    /// Cron schedule with seconds for carrying out approved imports.
    pub execute_imports: String,
    /// Cron schedule with seconds for checking whether Jellyfin should rescan.
    pub rescan_media_server: String,
    /// Cron schedule with seconds for refreshing the items due for it.
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
            refresh_metadata: "0 0 */12 * * *".into(),
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct WebConfig {
    /// Asset bundle directory; `assets/` beside the executable when unset.
    pub assets: Option<PathBuf>,
}

/// Serves the web interface, delivers events and runs scheduled jobs until SIGINT or SIGTERM.
pub async fn run(config: &Config) -> Result<()> {
    let schedules = config.serve.schedules()?;
    let server = Server::new(config.web.assets.as_deref())
        .context("Failed to load the web assets; bundle them with `topcoat asset bundle -p yokoku`")?;
    let listener = bind().await?;
    let address = listener.local_addr()?;
    let app = App::open(config).await?;

    let stop = CancellationToken::new();
    let shutdown = CancellationToken::new();
    let deliveries = app.spawn_deliveries(&shutdown);
    let web = tokio::spawn({
        let stop = stop.clone();
        async move {
            let result = server.run(listener, stop.clone().cancelled_owned()).await;
            stop.cancel();
            result
        }
    });
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
    info!(%address, "serving");
    let until_stopped = {
        let stop = stop.clone();
        async move {
            tokio::select! {
                result = stop_signal() => result,
                () = stop.cancelled() => Ok(()),
            }
        }
    };
    let result = monitor.run_with_signal(until_stopped).await;

    stop.cancel();
    let served = web.await.context("Web server panicked")?;
    shutdown.cancel();
    for delivery in deliveries {
        delivery.await.context("Event delivery failed")?;
    }
    info!("stopped");
    served.context("Web server failed")?;
    result.context("Jobs failed")
}

/// Listens on `HOST` and `PORT`, 127.0.0.1:3000 by default; `topcoat dev` sets both.
async fn bind() -> Result<TcpListener> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
    let port: u16 = port.parse().with_context(|| format!("Invalid PORT: {port}"))?;
    TcpListener::bind((host.as_str(), port)).await.with_context(|| format!("Failed to listen on {host}:{port}"))
}

async fn stop_signal() -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
