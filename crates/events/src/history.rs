use yokoku_domain::{ItemId, StorageError};

use crate::{EventId, EventLog, Recorded};

const BATCH: u32 = 200;

/// What happened, newest first (FR-9.1), read from the event log.
pub struct History {
    log: EventLog,
}

impl History {
    pub fn new(log: EventLog) -> Self {
        Self { log }
    }

    /// Up to `limit` events older than `before`, only those concerning `item` when given.
    pub async fn page(
        &self,
        item: Option<ItemId>,
        before: Option<EventId>,
        limit: usize,
    ) -> Result<Vec<Recorded>, StorageError> {
        let mut entries = Vec::new();
        let mut before = before;
        while entries.len() < limit {
            let batch = self.log.read_before(before, BATCH).await?;
            let Some(oldest) = batch.last() else { break };
            before = Some(oldest.id);
            let wanted =
                batch.into_iter().filter(|recorded| item.is_none_or(|item| recorded.event.items().contains(&item)));
            entries.extend(wanted.take(limit - entries.len()));
        }
        Ok(entries)
    }
}
