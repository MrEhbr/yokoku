//! apalis workers and schedules.

use std::sync::Arc;

use apalis::prelude::{BoxDynError, Data, Monitor, WorkerBuilder, WorkerBuilderExt};
use apalis_cron::{CronScheduler, Tick};
use jiff_cron::{Schedule, jiff::tz::TimeZone};
use tracing::info;
use yokoku_downloads::Downloads;
use yokoku_media::Importer;

/// Use cases the jobs call.
#[derive(Clone)]
pub struct Jobs {
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
}

/// Cron schedules, with seconds: `*/30 * * * * *` is every 30 seconds.
#[derive(Debug, Clone)]
pub struct Schedules {
    pub sync_downloads: Schedule,
    pub execute_imports: Schedule,
}

/// Registers every job, each running one tick at a time; run it with `Monitor::run_with_signal`.
pub fn monitor(jobs: Jobs, schedules: Schedules) -> Monitor {
    let cron = |schedule: &Schedule| CronScheduler::new(schedule.clone()).with_timezone(TimeZone::UTC);
    let (downloads, importer) = (jobs.downloads, jobs.importer);
    let (sync_schedule, import_schedule) = (schedules.sync_downloads, schedules.execute_imports);
    Monitor::new()
        .register(move |_| {
            WorkerBuilder::new("sync-downloads")
                .backend(cron(&sync_schedule))
                .concurrency(1)
                .data(downloads.clone())
                .build(sync_downloads)
        })
        .register(move |_| {
            WorkerBuilder::new("execute-imports")
                .backend(cron(&import_schedule))
                .concurrency(1)
                .data(importer.clone())
                .build(execute_imports)
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
