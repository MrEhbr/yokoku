use std::{sync::Arc, time::Duration};

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{EventLog, EventLogError, Failure, Listener, Recorded, Subscriber};

#[derive(Debug, Clone)]
pub struct DeliveryConfig {
    pub batch_size: u32,
    pub poll_interval: Duration,
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            poll_interval: Duration::from_secs(5),
            max_attempts: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
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
    log: Arc<dyn EventLog>,
    subscriber: Arc<dyn Subscriber>,
    listener: Listener,
    config: DeliveryConfig,
}

impl Delivery {
    pub fn new(
        log: Arc<dyn EventLog>,
        subscriber: Arc<dyn Subscriber>,
        listener: Listener,
        config: DeliveryConfig,
    ) -> Self {
        Self { log, subscriber, listener, config }
    }

    /// Runs until `shutdown` is cancelled; an interrupted event is redelivered on the next run.
    pub async fn run(self, shutdown: CancellationToken) {
        let subscriber = self.subscriber.name();
        info!(subscriber, "event delivery started");
        shutdown.run_until_cancelled(self.deliver_forever()).await;
        info!(subscriber, "event delivery stopped");
    }

    async fn deliver_forever(mut self) {
        loop {
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

    async fn deliver_batch(&self) -> Result<usize, EventLogError> {
        let after = self.log.last_delivered(self.subscriber.name()).await?;
        let batch = self.log.read_after(after, self.config.batch_size).await?;
        for recorded in &batch {
            self.deliver(recorded).await?;
        }
        Ok(batch.len())
    }

    async fn deliver(&self, recorded: &Recorded) -> Result<(), EventLogError> {
        let subscriber = self.subscriber.name();
        let mut attempt = 1;
        loop {
            match self.subscriber.handle(recorded).await {
                Ok(()) => return self.log.mark_delivered(subscriber, recorded.id).await,
                Err(error) if attempt >= self.config.max_attempts => {
                    error!(subscriber, event_id = %recorded.id, attempt, %error, "giving up on event");
                    let failure = Failure { event: recorded.id, error: error.to_string(), attempts: attempt };
                    return self.log.give_up(subscriber, &failure).await;
                },
                Err(error) => {
                    warn!(subscriber, event_id = %recorded.id, attempt, %error, "event handler failed");
                    sleep(self.config.backoff(attempt)).await;
                    attempt += 1;
                },
            }
        }
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
