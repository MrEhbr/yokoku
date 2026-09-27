use async_trait::async_trait;
use yokoku_domain::StorageError;

use crate::{Event, EventLog};

/// Keeps events the event log refused until it takes them.
#[async_trait]
pub trait EventSpool: Send + Sync {
    /// Adds `events` after those already spooled.
    async fn push(&self, events: &[Event]) -> Result<(), StorageError>;

    /// Appends every spooled event to `log`, oldest first, then empties the spool; returns how many.
    /// The spool is kept when `log` refuses them, and a stop between the two appends them again.
    async fn replay(&self, log: &dyn EventLog) -> Result<usize, StorageError>;
}
