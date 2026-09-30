use std::{error::Error, sync::Arc, time::Duration};

use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, error, info, info_span, warn};
use yokoku_domain::{CorrelationId, StorageError};

use crate::{DeliveryFailure, EventLog, Recorded, Subscription, correlation::correlate, signal::Listener};

#[derive(Debug, Clone)]
pub struct DeliveryConfig {
    pub batch_size: u32,
    pub poll_interval: Duration,
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    /// How often the delivery loop tries again the events it gave up on.
    pub retry_interval: Duration,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            poll_interval: Duration::from_secs(5),
            max_attempts: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            retry_interval: Duration::from_secs(600),
        }
    }
}

impl DeliveryConfig {
    /// Exponential backoff after the given failed attempt (1-based), capped at `max_backoff`.
    fn backoff(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        self.initial_backoff.saturating_mul(factor).min(self.max_backoff)
    }
}

/// Delivers the event log to one subscriber, in order, at least once.
pub struct Delivery {
    log: EventLog,
    subscriber: Arc<Subscription>,
    listener: Listener,
    config: DeliveryConfig,
}

impl Delivery {
    pub fn new(log: EventLog, subscriber: Arc<Subscription>, config: DeliveryConfig) -> Self {
        Self { listener: log.listen(), log, subscriber, config }
    }

    /// Runs until `shutdown` is cancelled; an interrupted event is redelivered on the next run.
    pub async fn run(self, shutdown: CancellationToken) {
        let subscriber = self.subscriber.name();
        info!(subscriber, "event delivery started");
        shutdown.run_until_cancelled(self.deliver_forever()).await;
        info!(subscriber, "event delivery stopped");
    }

    /// Tries again the events given up on earlier, then delivers every event already in the log;
    /// returns how many new events there were.
    pub async fn catch_up(&self) -> Result<usize, StorageError> {
        self.retry_failed().await?;
        let mut delivered = 0;
        loop {
            match self.deliver_batch().await? {
                0 => break,
                count => delivered += count,
            }
        }
        Ok(delivered)
    }

    async fn deliver_forever(mut self) {
        let mut last_retry = Instant::now();
        loop {
            if last_retry.elapsed() >= self.config.retry_interval {
                if let Err(error) = self.retry_failed().await {
                    warn!(subscriber = self.subscriber.name(), %error, "event log unavailable");
                }
                last_retry = Instant::now();
            }
            self.listener.mark_seen();
            match self.deliver_batch().await {
                Ok(0) => self.wait_for_events().await,
                Ok(_) => {},
                Err(error) => {
                    warn!(subscriber = self.subscriber.name(), %error, "event log unavailable");
                    sleep(self.config.poll_interval).await;
                },
            }
        }
    }

    /// Tries each event given up on once more; returns how many succeeded. They arrive after newer
    /// events, which idempotent handlers accept.
    pub async fn retry_failed(&self) -> Result<usize, StorageError> {
        let subscriber = self.subscriber.name();
        let mut resolved = 0;
        for (recorded, failure) in self.log.failed(subscriber).await? {
            let retried = async {
                match self.subscriber.handle(&recorded.event).await {
                    Ok(()) => {
                        info!("event handled on retry");
                        self.log.resolve(subscriber, recorded.id).await?;
                        Ok(true)
                    },
                    Err(error) => {
                        warn!(
                            attempt = failure.attempts + 1,
                            error = error.as_ref() as &(dyn Error + 'static),
                            "event handler failed again"
                        );
                        let failure =
                            DeliveryFailure { error: error.to_string(), attempts: failure.attempts + 1, ..failure };
                        self.log.record_failure(subscriber, &failure).await?;
                        Ok::<_, StorageError>(false)
                    },
                }
            };
            if self.scoped(&recorded, retried).await? {
                resolved += 1;
            }
        }
        Ok(resolved)
    }

    async fn deliver_batch(&self) -> Result<usize, StorageError> {
        let after = self.log.last_delivered(self.subscriber.name()).await?;
        let batch = self.log.read_after(after, self.config.batch_size).await?;
        for recorded in &batch {
            self.deliver(recorded).await?;
        }
        Ok(batch.len())
    }

    async fn deliver(&self, recorded: &Recorded) -> Result<(), StorageError> {
        self.scoped(recorded, self.deliver_attempts(recorded)).await
    }

    async fn deliver_attempts(&self, recorded: &Recorded) -> Result<(), StorageError> {
        let subscriber = self.subscriber.name();
        let mut attempt = 1;
        loop {
            let started = Instant::now();
            match self.subscriber.handle(&recorded.event).await {
                Ok(()) => {
                    debug!(elapsed_ms = started.elapsed().as_millis(), "event handled");
                    return self.log.mark_delivered(subscriber, recorded.id).await;
                },
                Err(error) if attempt >= self.config.max_attempts => {
                    error!(attempt, error = error.as_ref() as &(dyn Error + 'static), "giving up on event");
                    let failure = DeliveryFailure { event: recorded.id, error: error.to_string(), attempts: attempt };
                    return self.log.give_up(subscriber, &failure).await;
                },
                Err(error) => {
                    warn!(attempt, error = error.as_ref() as &(dyn Error + 'static), "event handler failed");
                    sleep(self.config.backoff(attempt)).await;
                    attempt += 1;
                },
            }
        }
    }

    /// Runs `work` in a root `deliver` span for `recorded`, under its correlation id or a new one.
    async fn scoped<T>(&self, recorded: &Recorded, work: impl Future<Output = T>) -> T {
        let correlation = recorded.correlation.unwrap_or_else(CorrelationId::generate);
        let span = info_span!(
            parent: None,
            "deliver",
            subscriber = self.subscriber.name(),
            event_id = %recorded.id,
            event = recorded.event.name(),
            %correlation,
        );
        correlate(correlation, work.instrument(span)).await
    }

    async fn wait_for_events(&mut self) {
        tokio::select! {
            () = self.listener.changed() => {},
            () = sleep(self.config.poll_interval) => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    fn config(initial_ms: u64, max_ms: u64) -> DeliveryConfig {
        DeliveryConfig {
            initial_backoff: Duration::from_millis(initial_ms),
            max_backoff: Duration::from_millis(max_ms),
            ..DeliveryConfig::default()
        }
    }

    #[rstest]
    #[case(1, 100)]
    #[case(2, 200)]
    #[case(3, 400)]
    #[case(5, 1_000)]
    #[case(u32::MAX, 1_000)]
    fn backoff_doubles_up_to_the_cap(#[case] attempt: u32, #[case] expected_ms: u64) {
        assert_eq!(config(100, 1_000).backoff(attempt), Duration::from_millis(expected_ms));
    }

    proptest! {
        #[test]
        fn backoff_stays_within_bounds_and_never_shrinks(
            initial_ms in 1..10_000u64,
            extra_ms in 0..100_000u64,
            attempt in 1..u32::MAX,
        ) {
            let config = config(initial_ms, initial_ms + extra_ms);
            let delay = config.backoff(attempt);

            prop_assert!(delay >= config.initial_backoff);
            prop_assert!(delay <= config.max_backoff);
            prop_assert!(config.backoff(attempt + 1) >= delay);
        }
    }
}
