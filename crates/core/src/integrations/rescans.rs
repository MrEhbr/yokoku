use std::sync::Arc;

use async_trait::async_trait;
use jiff::SignedDuration;
use tracing::{debug, info, instrument};
use yokoku_domain::{Clock, StorageError, events::Event};

use crate::{
    events::{HandlerError, Subscriber},
    integrations::ports::{MediaServer, MediaServerError, RescanStore},
    jobs::{Job, JobError},
};

/// Changes must stop arriving for this long before the media server rescans.
const QUIET: SignedDuration = SignedDuration::from_secs(30);

/// Tells the media server to rescan after library files change; a burst of changes
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

    /// Rescans now, pending request or not.
    #[instrument(skip_all)]
    pub async fn run_now(&self) -> Result<(), RescanError> {
        let requested_at = self.store.requested_at().await?;
        self.server.refresh_library().await?;
        info!("media server rescanning");
        if let Some(requested_at) = requested_at {
            self.store.clear(requested_at).await?;
        }
        Ok(())
    }
}

/// Requests a rescan when library files are imported, renamed or deleted.
#[async_trait]
impl Subscriber for Rescans {
    fn name(&self) -> &'static str {
        "integrations.rescans"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        if matches!(event, Event::FilesImported(_) | Event::FileRenamed(_) | Event::FileDeleted(_)) {
            self.store.request(self.clock.now().timestamp()).await?;
            debug!("media server rescan requested");
        }
        Ok(())
    }
}

/// Rescans when a request is pending and nothing newer arrived for `QUIET`. A request made while
/// the rescan runs, or while no media server is configured, stays pending.
#[async_trait]
impl Job for Rescans {
    fn name(&self) -> &'static str {
        "rescan-media-server"
    }

    #[instrument(skip_all)]
    async fn run(&self) -> Result<(), JobError> {
        let Some(requested_at) = self.store.requested_at().await? else { return Ok(()) };
        if self.clock.now().timestamp().duration_since(requested_at) < QUIET {
            return Ok(());
        }
        match self.server.refresh_library().await {
            Err(MediaServerError::NotConfigured) => return Ok(()),
            result => result?,
        }
        info!("media server rescanning");
        self.store.clear(requested_at).await?;
        Ok(())
    }
}
