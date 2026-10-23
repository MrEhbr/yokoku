use std::{sync::Arc, time::Duration};

use anyhow::{Result, bail};
use tokio::{task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use yokoku_core::{
    events::{Delivery, DeliveryConfig, EventLog, Publisher, Subscriber},
    jobs::{Job, Scheduler},
};
use yokoku_domain::StorageError;

use crate::config::Settings;

/// How often the events the log refused are appended again.
const FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// What runs without a caller: event deliveries to each subscriber and the scheduled jobs.
pub struct Background {
    log: EventLog,
    events: Publisher,
    subscribers: Vec<Arc<dyn Subscriber>>,
    delivery: DeliveryConfig,
    scheduler: Scheduler,
}

impl Background {
    /// Fails when a job has no schedule under `[serve]`.
    pub fn new(
        log: EventLog,
        events: Publisher,
        subscribers: Vec<Arc<dyn Subscriber>>,
        jobs: Vec<Arc<dyn Job>>,
        settings: &Settings,
    ) -> Result<Self> {
        let config = settings.current();
        if let Some(job) = jobs.iter().find(|job| config.serve.of(job.name()).is_none()) {
            bail!("No schedule for the {} job under [serve]", job.name());
        }
        let poll_interval = Duration::from_millis(config.events.poll_interval_ms);
        let delivery = DeliveryConfig { poll_interval, ..DeliveryConfig::default() };
        let scheduler = Scheduler::new(jobs, Arc::new(settings.clone()));
        Ok(Self { log, events, subscribers, delivery, scheduler })
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Starts one delivery loop per subscriber and one that appends kept events every
    /// `FLUSH_INTERVAL`; each stops when `shutdown` is cancelled.
    pub fn spawn_deliveries(&self, shutdown: &CancellationToken) -> JoinSet<()> {
        let mut deliveries: JoinSet<()> = self
            .subscribers
            .iter()
            .map(|subscriber| {
                Delivery::new(self.log.clone(), subscriber.clone(), self.delivery.clone()).run(shutdown.clone())
            })
            .collect();
        deliveries.spawn(flush_kept(self.events.clone(), shutdown.clone()));
        deliveries
    }

    /// Appends the events kept while the event log refused them.
    pub async fn flush(&self) -> Result<(), StorageError> {
        self.events.flush().await
    }
}

async fn flush_kept(events: Publisher, shutdown: CancellationToken) {
    shutdown
        .run_until_cancelled(async {
            loop {
                if let Err(error) = events.flush().await {
                    warn!(%error, "kept events are still waiting for the event log");
                }
                sleep(FLUSH_INTERVAL).await;
            }
        })
        .await;
}
