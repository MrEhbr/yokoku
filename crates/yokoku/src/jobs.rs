//! Scheduled jobs.

use std::{
    error::Error,
    str::FromStr,
    time::{Duration, Instant},
};

use jiff::{SignedDuration, Zoned, tz::TimeZone};
use jiff_cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, error, info, info_span, warn};
use yokoku_core::events::correlation::correlate;
use yokoku_domain::CorrelationId;

use crate::app::App;

/// Changes must stop arriving for this long before the media server rescans.
const RESCAN_QUIET: SignedDuration = SignedDuration::from_secs(30);

/// A job reads its schedule again at least this often.
const RECHECK: Duration = Duration::from_secs(1);

type BoxError = Box<dyn Error + Send + Sync>;

/// A cron schedule with seconds, such as `*/30 * * * * *` for every 30 seconds; written as its
/// expression.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Cron {
    expression: String,
    schedule: Schedule,
}

impl TryFrom<String> for Cron {
    type Error = InvalidSchedule;

    fn try_from(expression: String) -> Result<Self, Self::Error> {
        match Schedule::from_str(&expression) {
            Ok(schedule) => Ok(Self { expression, schedule }),
            Err(reason) => Err(InvalidSchedule { expression, reason }),
        }
    }
}

impl From<Cron> for String {
    fn from(cron: Cron) -> Self {
        cron.expression
    }
}

impl PartialEq for Cron {
    fn eq(&self, other: &Self) -> bool {
        self.expression == other.expression
    }
}

impl Eq for Cron {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScheduleSettings {
    /// Cron schedule with seconds for syncing downloads.
    pub sync_downloads: Cron,
    /// Cron schedule with seconds for syncing downloads while one is queued or downloading.
    pub sync_active_downloads: Cron,
    /// Cron schedule with seconds for carrying out approved imports.
    pub execute_imports: Cron,
    /// Cron schedule with seconds for checking whether Jellyfin should rescan.
    pub rescan_media_server: Cron,
    /// Cron schedule with seconds for syncing watched files from Jellyfin.
    pub sync_watched: Cron,
    /// Cron schedule with seconds for refreshing the items due for it.
    pub refresh_metadata: Cron,
    /// Cron schedule with seconds for scanning root folders for outside changes.
    pub scan_library: Cron,
}

impl Default for ScheduleSettings {
    fn default() -> Self {
        let cron = |expression: &str| Cron::try_from(expression.to_owned()).expect("the default schedules parse");
        Self {
            sync_downloads: cron("*/30 * * * * *"),
            sync_active_downloads: cron("*/5 * * * * *"),
            execute_imports: cron("*/5 * * * * *"),
            rescan_media_server: cron("*/10 * * * * *"),
            sync_watched: cron("0 */15 * * * *"),
            refresh_metadata: cron("0 0 */12 * * *"),
            scan_library: cron("0 0 5 * * *"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid schedule {expression:?}: {reason}")]
pub struct InvalidSchedule {
    expression: String,
    reason: jiff_cron::error::Error,
}

/// Starts each job on its schedule, one tick at a time; each stops when `shutdown` is cancelled,
/// after the tick it is running.
pub fn spawn(app: &App, shutdown: &CancellationToken) -> JoinSet<()> {
    Job::ALL.into_iter().map(|job| job.every(app.clone(), shutdown.clone())).collect()
}

/// A scheduled job; `[serve]` holds its schedule under the same name.
#[derive(Debug, Clone, Copy)]
enum Job {
    SyncDownloads,
    SyncActiveDownloads,
    ExecuteImports,
    ScanLibrary,
    RefreshMetadata,
    RescanMediaServer,
    SyncWatched,
}

impl Job {
    const ALL: [Self; 7] = [
        Self::SyncDownloads,
        Self::SyncActiveDownloads,
        Self::ExecuteImports,
        Self::ScanLibrary,
        Self::RefreshMetadata,
        Self::RescanMediaServer,
        Self::SyncWatched,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SyncDownloads => "sync-downloads",
            Self::SyncActiveDownloads => "sync-active-downloads",
            Self::ExecuteImports => "execute-imports",
            Self::ScanLibrary => "scan-library",
            Self::RefreshMetadata => "refresh-metadata",
            Self::RescanMediaServer => "rescan-media-server",
            Self::SyncWatched => "sync-watched",
        }
    }

    fn schedule(self, serve: &ScheduleSettings) -> &Cron {
        match self {
            Self::SyncDownloads => &serve.sync_downloads,
            Self::SyncActiveDownloads => &serve.sync_active_downloads,
            Self::ExecuteImports => &serve.execute_imports,
            Self::ScanLibrary => &serve.scan_library,
            Self::RefreshMetadata => &serve.refresh_metadata,
            Self::RescanMediaServer => &serve.rescan_media_server,
            Self::SyncWatched => &serve.sync_watched,
        }
    }

    async fn run(self, app: &App) -> Result<(), BoxError> {
        match self {
            Self::SyncDownloads => app.downloads.sync().await.map(drop)?,
            Self::SyncActiveDownloads => app.downloads.sync_active().await.map(drop)?,
            Self::ExecuteImports => app.importer.run_pending().await.map(drop)?,
            Self::ScanLibrary => app.scanner.scan().await.map(drop)?,
            Self::RefreshMetadata => {
                let report = app.metadata.refresh_due().await?;
                for failure in &report.failures {
                    warn!(item = ?failure.item, error = %failure.error, "metadata refresh failed");
                }
                info!(refreshed = report.refreshed, failed = report.failures.len(), "metadata refreshed");
            },
            Self::RescanMediaServer => app.rescans.run_due(RESCAN_QUIET).await.map(drop)?,
            Self::SyncWatched => app.watched.sync().await?,
        }
        Ok(())
    }

    /// Runs on each tick of the job's schedule in UTC, reading the schedule again before each tick
    /// and at least every `RECHECK`; a tick missed while the job runs is skipped.
    async fn every(self, app: App, shutdown: CancellationToken) {
        loop {
            let next = self.schedule(&app.settings.current().serve).schedule.upcoming(TimeZone::UTC).next();
            let wait = next.as_ref().map_or(RECHECK, |tick| {
                Duration::try_from(Zoned::now().duration_until(tick)).unwrap_or_default().min(RECHECK)
            });
            tokio::select! {
                biased;
                () = shutdown.cancelled() => return,
                () = sleep(wait) => {}
            }
            if next.is_some_and(|tick| Zoned::now() >= tick) {
                _ = run(self.name(), self.run(&app)).await;
            }
        }
    }
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
        ScheduleSettings::default();
    }
}
