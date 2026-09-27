use std::sync::Arc;

use tracing::{error, info, warn};

use crate::{Event, EventLog, EventSpool};

/// Appends events after the change they describe was saved. Events the log refuses go to the spool
/// and are appended, before any newer ones, once it takes them; events neither takes are logged
/// and lost, so their handlers never run.
#[derive(Clone)]
pub struct Publisher {
    log: Arc<dyn EventLog>,
    spool: Arc<dyn EventSpool>,
}

impl Publisher {
    pub fn new(log: Arc<dyn EventLog>, spool: Arc<dyn EventSpool>) -> Self {
        Self { log, spool }
    }

    pub async fn publish(&self, event: impl Into<Event>) {
        self.publish_all(vec![event.into()]).await;
    }

    pub async fn publish_all(&self, events: Vec<Event>) {
        if events.is_empty() {
            return;
        }
        if self.replay().await {
            match self.log.append(&events).await {
                Ok(()) => return,
                Err(error) => warn!(%error, "event log unavailable; spooling events"),
            }
        }
        if let Err(error) = self.spool.push(&events).await {
            error!(%error, ?events, "events of a saved change were lost; their handlers will not run");
        }
    }

    /// Appends the spooled events; `false` while some are still waiting.
    pub async fn replay(&self) -> bool {
        match self.spool.replay(self.log.as_ref()).await {
            Ok(0) => true,
            Ok(appended) => {
                info!(appended, "appended spooled events");
                true
            },
            Err(error) => {
                warn!(%error, "spooled events are still waiting for the event log");
                false
            },
        }
    }
}
