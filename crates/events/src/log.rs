use std::{error::Error, fmt};

use async_trait::async_trait;
use jiff::Timestamp;

use crate::Event;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub event: EventId,
    pub error: String,
    pub attempts: u32,
}

#[async_trait]
pub trait EventLog: Send + Sync {
    /// `None` until the subscriber's first delivery.
    async fn last_delivered(&self, subscriber: &str) -> Result<Option<EventId>, EventLogError>;

    /// Events with ids greater than `after`, in id order.
    async fn read_after(&self, after: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, EventLogError>;

    /// Events with ids less than `before`, newest first; the newest events when `before` is `None`.
    async fn read_before(&self, before: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, EventLogError>;

    async fn mark_delivered(&self, subscriber: &str, event: EventId) -> Result<(), EventLogError>;

    /// Records the failure and moves the subscriber past the event, atomically.
    async fn give_up(&self, subscriber: &str, failure: &Failure) -> Result<(), EventLogError>;

    /// Events the subscriber gave up on, with their failures, oldest first.
    async fn failed(&self, subscriber: &str) -> Result<Vec<(Recorded, Failure)>, EventLogError>;

    /// Updates a failure after another attempt; the position stays where it is.
    async fn record_failure(&self, subscriber: &str, failure: &Failure) -> Result<(), EventLogError>;

    /// Forgets a failure once the event was handled.
    async fn resolve(&self, subscriber: &str, event: EventId) -> Result<(), EventLogError>;
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct EventLogError(Box<dyn Error + Send + Sync>);

impl EventLogError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
