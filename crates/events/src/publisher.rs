use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, warn};
use yokoku_domain::StorageError;

use crate::{Correlated, CorrelationId, Event, EventLog, correlation};

/// Appends events after the change they describe was saved. Events the log refuses are kept in
/// memory and appended, before any newer ones, by the next publish or `flush`; they are lost if the
/// process stops first. Clones share the kept events. Each event carries the current correlation id,
/// or a new one outside any.
#[derive(Clone)]
pub struct Publisher {
    log: Arc<dyn EventLog>,
    waiting: Arc<Mutex<Vec<Correlated>>>,
}

impl Publisher {
    pub fn new(log: Arc<dyn EventLog>) -> Self {
        Self { log, waiting: Arc::default() }
    }

    pub async fn publish(&self, event: impl Into<Event>) {
        self.publish_all(vec![event.into()]).await;
    }

    pub async fn publish_all(&self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        let correlation = correlation::current().unwrap_or_else(CorrelationId::generate);
        let mut waiting = self.waiting.lock().await;
        waiting.extend(events.into_iter().map(|event| Correlated { correlation, event }));
        if let Err(error) = self.append(&mut waiting).await {
            warn!(%error, waiting = waiting.len(), "event log unavailable; keeping events to append later");
        }
    }

    /// Appends the kept events; they stay kept while the log refuses them.
    pub async fn flush(&self) -> Result<(), StorageError> {
        self.append(&mut *self.waiting.lock().await).await
    }

    async fn append(&self, waiting: &mut Vec<Correlated>) -> Result<(), StorageError> {
        if waiting.is_empty() {
            return Ok(());
        }
        self.log.append(waiting).await?;
        debug!(events = ?waiting.iter().map(|event| event.event.name()).collect::<Vec<_>>(), "published");
        waiting.clear();
        Ok(())
    }
}
