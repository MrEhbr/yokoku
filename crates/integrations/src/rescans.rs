use std::sync::Arc;

use async_trait::async_trait;
use jiff::SignedDuration;
use yokoku_domain::{Clock, StorageError};
use yokoku_events::{FileDeleted, FileRenamed, FilesImported, HandlerError, Recorded, Subscriber};

use crate::ports::{MediaServer, MediaServerError, RescanStore};

/// Tells the media server to rescan after library files change (FR-10.4); a burst of changes
/// leads to one rescan.
pub struct Rescans {
    store: Arc<dyn RescanStore>,
    server: Arc<dyn MediaServer>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, thiserror::Error)]
pub enum RescanError {
    #[error(transparent)]
    Server(#[from] MediaServerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl Rescans {
    pub fn new(store: Arc<dyn RescanStore>, server: Arc<dyn MediaServer>, clock: Arc<dyn Clock>) -> Self {
        Self { store, server, clock }
    }

    pub async fn test_connection(&self) -> Result<String, RescanError> {
        Ok(self.server.version().await?)
    }

    /// Rescans when a request is pending and nothing newer arrived for `quiet`; returns whether it
    /// did. A request made while the rescan runs stays pending.
    pub async fn run_due(&self, quiet: SignedDuration) -> Result<bool, RescanError> {
        let Some(requested_at) = self.store.requested_at().await? else { return Ok(false) };
        if self.clock.now().timestamp().duration_since(requested_at) < quiet {
            return Ok(false);
        }
        self.server.refresh_library().await?;
        self.store.clear(requested_at).await?;
        Ok(true)
    }

    /// Rescans now, pending request or not.
    pub async fn run_now(&self) -> Result<(), RescanError> {
        let requested_at = self.store.requested_at().await?;
        self.server.refresh_library().await?;
        if let Some(requested_at) = requested_at {
            self.store.clear(requested_at).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Subscriber for Rescans {
    fn name(&self) -> &'static str {
        "integrations.rescans"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        let event = &recorded.event;
        if event.get::<FilesImported>().is_some()
            || event.get::<FileRenamed>().is_some()
            || event.get::<FileDeleted>().is_some()
        {
            self.store.request(self.clock.now().timestamp()).await?;
        }
        Ok(())
    }
}
