//! apalis workers and schedules.

use std::{sync::Arc, time::Instant};

use apalis::prelude::{BoxDynError, Data, Monitor, WorkerBuilder, WorkerBuilderExt};
use apalis_cron::{CronScheduler, Tick};
use jiff::SignedDuration;
use jiff_cron::{Schedule, jiff::tz::TimeZone};
use tracing::{Instrument, debug, error, info, info_span, warn};
use yokoku_downloads::Downloads;
use yokoku_integrations::Rescans;
use yokoku_library::MetadataSync;
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
    pub metadata: Option<Arc<MetadataSync>>,
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

/// Runs one tick in a `job` span, logging its duration and any failure.
async fn run(name: &'static str, job: impl Future<Output = Result<(), BoxDynError>>) -> Result<(), BoxDynError> {
    async {
        let started = Instant::now();
        let result = job.await;
        let elapsed_ms = started.elapsed().as_millis();
        match &result {
            Ok(()) => debug!(elapsed_ms, "job finished"),
            Err(error) => error!(%error, elapsed_ms, "job failed"),
        }
        result
    }
    .instrument(info_span!("job", name))
    .await
}

async fn sync_downloads(_tick: Tick<TimeZone>, downloads: Data<Arc<Downloads>>) -> Result<(), BoxDynError> {
    let report = downloads.sync().await?;
    if !report.completed.is_empty() {
        info!(completed = report.completed.len(), "downloads finished");
    }
    Ok(())
}

async fn execute_imports(_tick: Tick<TimeZone>, importer: Data<Arc<Importer>>) -> Result<(), BoxDynError> {
    for import in importer.run_pending().await? {
        info!(import = %import.id, status = ?import.status, "import finished");
    }
    Ok(())
}

async fn scan_library(_tick: Tick<TimeZone>, scanner: Data<Arc<Scanner>>) -> Result<(), BoxDynError> {
    let report = scanner.scan().await?;
    if report.found > 0 || report.vanished > 0 || !report.needs_review.is_empty() {
        info!(
            found = report.found,
            vanished = report.vanished,
            needs_review = report.needs_review.len(),
            "library files changed outside the app"
        );
    }
    Ok(())
}

async fn refresh_metadata(_tick: Tick<TimeZone>, metadata: Data<Arc<MetadataSync>>) -> Result<(), BoxDynError> {
    let report = metadata.refresh_all().await?;
    for failure in &report.failures {
        warn!(item = ?failure.item, error = %failure.error, "metadata refresh failed");
    }
    info!(refreshed = report.refreshed, failed = report.failures.len(), "metadata refreshed");
    Ok(())
}

async fn rescan_media_server(_tick: Tick<TimeZone>, rescans: Data<Arc<Rescans>>) -> Result<(), BoxDynError> {
    if rescans.run_due(RESCAN_QUIET).await? {
        info!("media server rescanning");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{Arc, Mutex},
    };

    use super::run;

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
    async fn a_failed_job_is_logged_in_its_span() {
        let captured = Captured::default();
        let writer = captured.clone();
        let subscriber = tracing_subscriber::fmt().with_writer(move || writer.clone()).with_ansi(false).finish();
        let _default = tracing::subscriber::set_default(subscriber);

        let result = run("sync-downloads", async { Err("transmission is unreachable".into()) }).await;

        assert!(result.is_err());
        let logs = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("ERROR job{name=\"sync-downloads\"}"), "{logs}");
        assert!(logs.contains("job failed error=transmission is unreachable"), "{logs}");
    }
}
