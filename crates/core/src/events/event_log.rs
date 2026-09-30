use std::{fmt, sync::Arc};

use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_domain::{CorrelationId, StorageError};

use crate::events::{
    Event,
    signal::{Listener, NewEvents},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(pub i64);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub id: EventId,
    pub occurred_at: Timestamp,
    pub event: Event,
    /// `None` for events stored before correlation ids.
    pub correlation: Option<CorrelationId>,
}

/// An event with the correlation id of the command, job or delivery that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correlated {
    pub correlation: CorrelationId,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryFailure {
    pub event: EventId,
    pub error: String,
    pub attempts: u32,
}

/// The events, each subscriber's position and the deliveries given up on.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Appends `events` in one transaction, in order.
    async fn append(&self, events: &[Correlated]) -> Result<(), StorageError>;

    /// `None` until the subscriber's first delivery.
    async fn last_delivered(&self, subscriber: &str) -> Result<Option<EventId>, StorageError>;

    /// Events with ids greater than `after`, in id order.
    async fn read_after(&self, after: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError>;

    /// Events with ids less than `before`, newest first; the newest events when `before` is `None`.
    async fn read_before(&self, before: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError>;

    async fn mark_delivered(&self, subscriber: &str, event: EventId) -> Result<(), StorageError>;

    /// Records the failure and moves the subscriber past the event, atomically.
    async fn give_up(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError>;

    /// Events the subscriber gave up on, with their failures, oldest first.
    async fn failed(&self, subscriber: &str) -> Result<Vec<(Recorded, DeliveryFailure)>, StorageError>;

    /// Updates a failure after another attempt; the position stays where it is.
    async fn record_failure(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError>;

    /// Forgets a failure once the event was handled.
    async fn resolve(&self, subscriber: &str, event: EventId) -> Result<(), StorageError>;
}

/// An `EventStore` that wakes event deliveries after each append. Clones share the wake-up.
#[derive(Clone)]
pub struct EventLog {
    store: Arc<dyn EventStore>,
    new_events: NewEvents,
}

impl EventLog {
    pub fn new(store: impl EventStore + 'static) -> Self {
        Self { store: Arc::new(store), new_events: NewEvents::new() }
    }

    /// Wakes after each append through this log or a clone of it.
    pub(crate) fn listen(&self) -> Listener {
        self.new_events.listen()
    }

    /// Appends `events` in one transaction, in order, then wakes event deliveries.
    pub async fn append(&self, events: &[Correlated]) -> Result<(), StorageError> {
        self.store.append(events).await?;
        if !events.is_empty() {
            self.new_events.notify();
        }
        Ok(())
    }

    pub async fn last_delivered(&self, subscriber: &str) -> Result<Option<EventId>, StorageError> {
        self.store.last_delivered(subscriber).await
    }

    pub async fn read_after(&self, after: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError> {
        self.store.read_after(after, limit).await
    }

    pub async fn read_before(&self, before: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError> {
        self.store.read_before(before, limit).await
    }

    pub async fn mark_delivered(&self, subscriber: &str, event: EventId) -> Result<(), StorageError> {
        self.store.mark_delivered(subscriber, event).await
    }

    pub async fn give_up(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError> {
        self.store.give_up(subscriber, failure).await
    }

    pub async fn failed(&self, subscriber: &str) -> Result<Vec<(Recorded, DeliveryFailure)>, StorageError> {
        self.store.failed(subscriber).await
    }

    pub async fn record_failure(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError> {
        self.store.record_failure(subscriber, failure).await
    }

    pub async fn resolve(&self, subscriber: &str, event: EventId) -> Result<(), StorageError> {
        self.store.resolve(subscriber, event).await
    }
}
