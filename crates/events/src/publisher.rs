use std::sync::Arc;

use tracing::error;

use crate::{Event, EventLog};

/// Appends events after the change they describe was saved. A failure is logged and the events are
/// lost, so their handlers never run.
#[derive(Clone)]
pub struct Publisher {
    log: Arc<dyn EventLog>,
}

impl Publisher {
    pub fn new(log: Arc<dyn EventLog>) -> Self {
        Self { log }
    }

    pub async fn publish(&self, event: impl Into<Event>) {
        self.publish_all(vec![event.into()]).await;
    }

    pub async fn publish_all(&self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        if let Err(error) = self.log.append(&events).await {
            error!(%error, ?events, "events of a saved change were lost; their handlers will not run");
        }
    }
}
