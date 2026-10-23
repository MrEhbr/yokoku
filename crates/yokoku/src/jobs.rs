//! Every scheduled job in the system; `[serve]` holds each job's schedule under its name.

use std::sync::Arc;

use anyhow::{Result, bail};
use yokoku_core::{
    downloads::SyncActiveDownloads,
    jobs::{Job, Scheduler},
};

use crate::app::App;

impl App {
    /// Fails when a job has no schedule.
    pub(crate) fn scheduler(&self) -> Result<Scheduler> {
        let jobs: Vec<Arc<dyn Job>> = vec![
            self.downloads.clone(),
            Arc::new(SyncActiveDownloads::new(self.downloads.clone())),
            self.importer.clone(),
            self.scanner.clone(),
            self.metadata.clone(),
            self.rescans.clone(),
            self.watched.clone(),
            self.ratings.clone(),
        ];
        let schedules = &self.settings.current().serve;
        if let Some(job) = jobs.iter().find(|job| schedules.of(job.name()).is_none()) {
            bail!("No schedule for the {} job under [serve]", job.name());
        }
        Ok(Scheduler::new(jobs, Arc::new(self.settings.clone())))
    }
}
