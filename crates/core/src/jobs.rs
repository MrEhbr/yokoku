//! Scheduled jobs and the scheduler that runs each on its schedule.

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use jiff::Zoned;
use tokio::{task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, error, info_span};
use yokoku_domain::CorrelationId;

use crate::events::correlation::correlate;

pub type JobError = Box<dyn Error + Send + Sync>;

/// A job reads its schedule again at least this often.
const RECHECK: Duration = Duration::from_secs(1);

#[async_trait]
pub trait Job: Send + Sync {
    /// Kebab-case; names the job on the command line and in logs.
    fn name(&self) -> &'static str;

    async fn run(&self) -> Result<(), JobError>;
}

/// When each job runs next.
pub trait Schedules: Send + Sync {
    /// `None` while `job` has no upcoming tick.
    fn next_tick(&self, job: &str) -> Option<Zoned>;
}

/// Runs each job on each tick of its schedule, one tick at a time.
pub struct Scheduler {
    jobs: Vec<Arc<dyn Job>>,
    schedules: Arc<dyn Schedules>,
}

impl Scheduler {
    pub fn new(jobs: Vec<Arc<dyn Job>>, schedules: Arc<dyn Schedules>) -> Self {
        Self { jobs, schedules }
    }

    pub fn jobs(&self) -> &[Arc<dyn Job>] {
        &self.jobs
    }

    pub fn job(&self, name: &str) -> Option<&Arc<dyn Job>> {
        self.jobs.iter().find(|job| job.name() == name)
    }

    /// Starts every job; each stops when `shutdown` is cancelled, after the tick it is running.
    pub fn spawn(&self, shutdown: &CancellationToken) -> JoinSet<()> {
        self.jobs.iter().map(|job| every(job.clone(), self.schedules.clone(), shutdown.clone())).collect()
    }
}

/// Runs on each tick of the job's schedule, reading the schedule again before each tick and at least
/// every `RECHECK`; a tick missed while the job runs is skipped.
async fn every(job: Arc<dyn Job>, schedules: Arc<dyn Schedules>, shutdown: CancellationToken) {
    loop {
        let next = schedules.next_tick(job.name());
        let wait = next.as_ref().map_or(RECHECK, |tick| {
            Duration::try_from(Zoned::now().duration_until(tick)).unwrap_or_default().min(RECHECK)
        });
        tokio::select! {
            biased;
            () = shutdown.cancelled() => return,
            () = sleep(wait) => {}
        }
        if next.is_some_and(|tick| Zoned::now() >= tick) {
            _ = run(job.name(), job.run()).await;
        }
    }
}

/// Runs one tick in a root `job` span under a new correlation id, logging its duration and any failure.
async fn run(name: &str, job: impl Future<Output = Result<(), JobError>>) -> Result<(), JobError> {
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
