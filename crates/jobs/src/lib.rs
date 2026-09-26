//! apalis workers and schedules.

use std::sync::Arc;

use apalis::prelude::{BoxDynError, Data, Monitor, WorkerBuilder, WorkerBuilderExt};
use apalis_cron::{CronScheduler, Tick};
use jiff::SignedDuration;
use jiff_cron::{Schedule, jiff::tz::TimeZone};
use tracing::info;
use yokoku_downloads::Downloads;
use yokoku_integrations::Rescans;
use yokoku_media::{Deleter, Importer};

/// Changes must stop arriving for this long before the media server rescans.
const RESCAN_QUIET: SignedDuration = SignedDuration::from_secs(30);

/// Use cases the jobs call.
#[derive(Clone)]
pub struct Jobs {
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
    pub deleter: Arc<Deleter>,
    /// `None` while no media server is configured.
    pub rescans: Option<Arc<Rescans>>,
}

/// Cron schedules, with seconds: `*/30 * * * * *` is every 30 seconds.
#[derive(Debug, Clone)]
pub struct Schedules {
    pub sync_downloads: Schedule,
    pub execute_imports: Schedule,
    pub cleanup_recycle: Schedule,
    pub rescan_media_server: Schedule,
}

/// Registers every job, each running one tick at a time; run it with `Monitor::run_with_signal`.
pub fn monitor(jobs: Jobs, schedules: Schedules) -> Monitor {
    let monitor = Monitor::new();
    let monitor = register(monitor, "sync-downloads", schedules.sync_downloads, jobs.downloads, sync_downloads);
    let monitor = register(monitor, "execute-imports", schedules.execute_imports, jobs.importer, execute_imports);
    let monitor = register(monitor, "cleanup-recycle", schedules.cleanup_recycle, jobs.deleter, cleanup_recycle);
    match jobs.rescans {
        Some(rescans) => {
            register(monitor, "rescan-media-server", schedules.rescan_media_server, rescans, rescan_media_server)
        },
        None => monitor,
    }
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
            .build(job.clone())
    })
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

async fn cleanup_recycle(_tick: Tick<TimeZone>, deleter: Data<Arc<Deleter>>) -> Result<(), BoxDynError> {
    let removed = deleter.clean_recycle().await?;
    if removed > 0 {
        info!(removed, "removed old recycled files");
    }
    Ok(())
}

async fn rescan_media_server(_tick: Tick<TimeZone>, rescans: Data<Arc<Rescans>>) -> Result<(), BoxDynError> {
    if rescans.run_due(RESCAN_QUIET).await? {
        info!("media server rescanning");
    }
    Ok(())
}
