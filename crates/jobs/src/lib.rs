//! apalis workers and schedules.

use std::sync::Arc;

use apalis::prelude::{BoxDynError, Data, Monitor, WorkerBuilder};
use apalis_cron::{CronScheduler, Tick};
use jiff_cron::{Schedule, jiff::tz::TimeZone};
use tracing::info;
use yokoku_downloads::Downloads;

/// Use cases the jobs call.
#[derive(Clone)]
pub struct Jobs {
    pub downloads: Arc<Downloads>,
}

/// Cron schedules, with seconds: `*/30 * * * * *` is every 30 seconds.
#[derive(Debug, Clone)]
pub struct Schedules {
    pub sync_downloads: Schedule,
}

/// Registers every job; run it with `Monitor::run_with_signal`.
pub fn monitor(jobs: Jobs, schedules: Schedules) -> Monitor {
    Monitor::new().register(move |_| {
        WorkerBuilder::new("sync-downloads")
            .backend(CronScheduler::new(schedules.sync_downloads.clone()).with_timezone(TimeZone::UTC))
            .data(jobs.downloads.clone())
            .build(sync_downloads)
    })
}

async fn sync_downloads(_tick: Tick<TimeZone>, downloads: Data<Arc<Downloads>>) -> Result<(), BoxDynError> {
    let report = downloads.sync().await?;
    if !report.completed.is_empty() {
        info!(completed = report.completed.len(), "downloads finished");
    }
    Ok(())
}
