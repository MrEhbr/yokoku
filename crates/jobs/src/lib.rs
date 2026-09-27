//! apalis workers and schedules.

use std::{error::Error, str::FromStr, sync::Arc, time::Instant};

use apalis::prelude::{BoxDynError, Data, Monitor, WorkerBuilder, WorkerBuilderExt};
use apalis_cron::{CronScheduler, Tick};
use jiff::SignedDuration;
use jiff_cron::{Schedule, jiff::tz::TimeZone};
use serde::{Deserialize, Serialize};
use tracing::{Instrument, debug, error, info, info_span, warn};
use yokoku_downloads::Downloads;
use yokoku_events::{CorrelationId, correlation::correlate};
use yokoku_integrations::Rescans;
use yokoku_library::MetadataService;
use yokoku_media::{Importer, Scanner};

/// Changes must stop arriving for this long before the media server rescans.
const RESCAN_QUIET: SignedDuration = SignedDuration::from_secs(30);

/// Use cases the jobs call.
#[derive(Clone)]
pub struct Jobs {
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
    pub scanner: Arc<Scanner>,
    /// `None` while no metadata source is configured.
    pub metadata: Option<Arc<MetadataService>>,
    /// `None` while no media server is configured.
    pub rescans: Option<Arc<Rescans>>,
}

/// Cron schedules, with seconds: `*/30 * * * * *` is every 30 seconds.
#[derive(Debug, Clone)]
pub struct Schedules {
    pub sync_downloads: Schedule,
    pub execute_imports: Schedule,
    pub rescan_media_server: Schedule,
    pub refresh_metadata: Schedule,
    pub scan_library: Schedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScheduleSettings {
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

impl Default for ScheduleSettings {
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

#[derive(Debug, thiserror::Error)]
#[error("Invalid schedule: {expression}")]
pub struct InvalidSchedule {
    expression: String,
    #[source]
    source: jiff_cron::error::Error,
}

impl ScheduleSettings {
    pub fn schedules(&self) -> Result<Schedules, InvalidSchedule> {
        Ok(Schedules {
            sync_downloads: schedule(&self.sync_downloads)?,
            execute_imports: schedule(&self.execute_imports)?,
            rescan_media_server: schedule(&self.rescan_media_server)?,
            refresh_metadata: schedule(&self.refresh_metadata)?,
            scan_library: schedule(&self.scan_library)?,
        })
    }
}

fn schedule(expression: &str) -> Result<Schedule, InvalidSchedule> {
    Schedule::from_str(expression).map_err(|source| InvalidSchedule { expression: expression.to_owned(), source })
}

/// Registers every job, each running one tick at a time; run it with `Monitor::run_with_signal`.
pub fn monitor(jobs: Jobs, schedules: Schedules) -> Monitor {
    let mut monitor = Monitor::new();
    monitor = register(monitor, "sync-downloads", schedules.sync_downloads, jobs.downloads, sync_downloads);
    monitor = register(monitor, "execute-imports", schedules.execute_imports, jobs.importer, execute_imports);
    monitor = register(monitor, "scan-library", schedules.scan_library, jobs.scanner, scan_library);
    if let Some(metadata) = jobs.metadata {
        monitor = register(monitor, "refresh-metadata", schedules.refresh_metadata, metadata, refresh_metadata);
    }
    if let Some(rescans) = jobs.rescans {
        monitor = register(monitor, "rescan-media-server", schedules.rescan_media_server, rescans, rescan_media_server);
    }
    monitor
}

fn register<T, F, Fut>(monitor: Monitor, name: &'static str, schedule: Schedule, data: Arc<T>, job: F) -> Monitor
where
    T: Send + Sync + 'static,
    F: Fn(Tick<TimeZone>, Data<Arc<T>>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<(), BoxDynError>> + Send + 'static,
{
    monitor.register(move |_| {
        WorkerBuilder::new(name)
            .backend(CronScheduler::new(schedule.clone()).with_timezone(TimeZone::UTC))
            .concurrency(1)
            .data(data.clone())
            .build({
                let job = job.clone();
                move |tick: Tick<TimeZone>, data: Data<Arc<T>>| run(name, job(tick, data))
            })
    })
}

/// Runs one tick in a root `job` span under a new correlation id, logging its duration and any failure.
async fn run(name: &'static str, job: impl Future<Output = Result<(), BoxDynError>>) -> Result<(), BoxDynError> {
    let correlation = CorrelationId::generate();
    let work = async {
        let started = Instant::now();
        let result = job.await;
        let elapsed_ms = started.elapsed().as_millis();
        match &result {
            Ok(()) => debug!(elapsed_ms, "job finished"),
            Err(error) => error!(error = error.as_ref() as &(dyn Error + 'static), elapsed_ms, "job failed"),
        }
        result
    };
    correlate(correlation, work.instrument(info_span!(parent: None, "job", name, %correlation))).await
}

async fn sync_downloads(_tick: Tick<TimeZone>, downloads: Data<Arc<Downloads>>) -> Result<(), BoxDynError> {
    downloads.sync().await?;
    Ok(())
}

async fn execute_imports(_tick: Tick<TimeZone>, importer: Data<Arc<Importer>>) -> Result<(), BoxDynError> {
    importer.run_pending().await?;
    Ok(())
}

async fn scan_library(_tick: Tick<TimeZone>, scanner: Data<Arc<Scanner>>) -> Result<(), BoxDynError> {
    scanner.scan().await?;
    Ok(())
}

async fn refresh_metadata(_tick: Tick<TimeZone>, metadata: Data<Arc<MetadataService>>) -> Result<(), BoxDynError> {
    let report = metadata.refresh_due().await?;
    for failure in &report.failures {
        warn!(item = ?failure.item, error = %failure.error, "metadata refresh failed");
    }
    info!(refreshed = report.refreshed, failed = report.failures.len(), "metadata refreshed");
    Ok(())
}

async fn rescan_media_server(_tick: Tick<TimeZone>, rescans: Data<Arc<Rescans>>) -> Result<(), BoxDynError> {
    rescans.run_due(RESCAN_QUIET).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fmt, io,
        sync::{Arc, Mutex},
    };

    use tracing::{Instrument, info_span};

    use super::run;

    /// Fails because of `source`.
    #[derive(Debug)]
    struct Unavailable(io::Error);

    impl fmt::Display for Unavailable {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("download client unavailable")
        }
    }

    impl Error for Unavailable {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_failed_job_is_logged_with_its_cause_in_a_root_span() {
        let captured = Captured::default();
        let writer = captured.clone();
        let subscriber = tracing_subscriber::fmt().with_writer(move || writer.clone()).with_ansi(false).finish();
        let _default = tracing::subscriber::set_default(subscriber);

        let refused = Unavailable(io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused"));
        let result = run("sync-downloads", async { Err(refused.into()) }).instrument(info_span!("command")).await;

        assert!(result.is_err());
        let logs = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("ERROR job{name=\"sync-downloads\" correlation="), "{logs}");
        assert!(logs.contains("error=download client unavailable error.sources=[connection refused]"), "{logs}");
    }
}
