//! Scheduled jobs.

use std::{
    error::Error,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use jiff::{SignedDuration, Zoned, tz::TimeZone};
use jiff_cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, error, info, info_span, warn};
use yokoku_core::{
    downloads::Downloads,
    events::correlation::correlate,
    integrations::Rescans,
    library::MetadataService,
    media::{Importer, Scanner},
};
use yokoku_domain::{CorrelationId, Live};

/// Changes must stop arriving for this long before the media server rescans.
const RESCAN_QUIET: SignedDuration = SignedDuration::from_secs(30);

/// A job reads its schedule again at least this often.
const RECHECK: Duration = Duration::from_secs(1);

type BoxError = Box<dyn Error + Send + Sync>;

/// Use cases the jobs call.
#[derive(Clone)]
pub struct Jobs {
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
    pub scanner: Arc<Scanner>,
    pub metadata: Arc<MetadataService>,
    pub rescans: Arc<Rescans>,
}

/// Cron schedules, with seconds: `*/30 * * * * *` is every 30 seconds.
#[derive(Debug, Clone)]
pub struct Schedules {
    pub sync_downloads: Schedule,
    pub sync_active_downloads: Schedule,
    pub execute_imports: Schedule,
    pub rescan_media_server: Schedule,
    pub refresh_metadata: Schedule,
    pub scan_library: Schedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScheduleSettings {
    /// Cron schedule with seconds for syncing downloads.
    pub sync_downloads: String,
    /// Cron schedule with seconds for syncing downloads while one is queued or downloading.
    pub sync_active_downloads: String,
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
            sync_active_downloads: "*/5 * * * * *".into(),
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
            sync_active_downloads: schedule(&self.sync_active_downloads)?,
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

/// Starts each job on its schedule, one tick at a time; each stops when `shutdown` is cancelled,
/// after the tick it is running.
pub fn spawn(
    jobs: Jobs,
    schedules: Live<Result<Schedules, InvalidSchedule>>,
    shutdown: &CancellationToken,
) -> Vec<JoinHandle<()>> {
    let Jobs { downloads, importer, scanner, metadata, rescans } = jobs;
    let active = downloads.clone();
    let shutdown = || shutdown.clone();
    let schedule = |pick: fn(Schedules) -> Schedule| {
        let schedules = schedules.clone();
        move || schedules.current().map(pick)
    };
    vec![
        tokio::spawn(every("sync-downloads", schedule(|s| s.sync_downloads), shutdown(), async move || {
            downloads.sync().await
        })),
        tokio::spawn(every(
            "sync-active-downloads",
            schedule(|s| s.sync_active_downloads),
            shutdown(),
            async move || active.sync_active().await,
        )),
        tokio::spawn(every("execute-imports", schedule(|s| s.execute_imports), shutdown(), async move || {
            importer.run_pending().await
        })),
        tokio::spawn(every("scan-library", schedule(|s| s.scan_library), shutdown(), async move || {
            scanner.scan().await
        })),
        tokio::spawn(every("refresh-metadata", schedule(|s| s.refresh_metadata), shutdown(), async move || {
            refresh_metadata(&metadata).await
        })),
        tokio::spawn(every("rescan-media-server", schedule(|s| s.rescan_media_server), shutdown(), async move || {
            rescans.run_due(RESCAN_QUIET).await
        })),
    ]
}

/// Runs `job` on each tick of `schedule` in UTC, reading `schedule` again before each tick and at
/// least every `RECHECK`; a tick missed while `job` runs is skipped.
async fn every<T, E: Into<BoxError>>(
    name: &'static str,
    schedule: impl Fn() -> Result<Schedule, InvalidSchedule>,
    shutdown: CancellationToken,
    job: impl AsyncFn() -> Result<T, E>,
) {
    loop {
        let next = schedule()
            .inspect_err(|error| error!(job = name, error = error as &(dyn Error + 'static), "job not scheduled"))
            .ok()
            .and_then(|schedule| schedule.upcoming(TimeZone::UTC).next());
        let wait = next.as_ref().map_or(RECHECK, |tick| until(tick).min(RECHECK));
        tokio::select! {
            biased;
            () = shutdown.cancelled() => return,
            () = sleep(wait) => {}
        }
        if next.is_some_and(|tick| Zoned::now() >= tick) {
            _ = run(name, async { job().await.map(drop).map_err(Into::into) }).await;
        }
    }
}

/// Zero once `tick` has passed.
fn until(tick: &Zoned) -> Duration {
    Zoned::now().duration_until(tick).try_into().unwrap_or_default()
}

/// Runs one tick in a root `job` span under a new correlation id, logging its duration and any failure.
async fn run(name: &'static str, job: impl Future<Output = Result<(), BoxError>>) -> Result<(), BoxError> {
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

async fn refresh_metadata(metadata: &MetadataService) -> Result<(), BoxError> {
    let report = metadata.refresh_due().await?;
    for failure in &report.failures {
        warn!(item = ?failure.item, error = %failure.error, "metadata refresh failed");
    }
    info!(refreshed = report.refreshed, failed = report.failures.len(), "metadata refreshed");
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

    use super::{ScheduleSettings, run};

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

    #[test]
    fn the_default_schedules_parse() {
        ScheduleSettings::default().schedules().unwrap();
    }
}
