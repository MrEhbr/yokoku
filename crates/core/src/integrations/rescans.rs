use std::sync::Arc;

use async_trait::async_trait;
use jiff::SignedDuration;
use tracing::{debug, info, instrument};
use yokoku_domain::{Clock, StorageError};

use crate::{
    events::{FileDeleted, FileRenamed, FilesImported, Handler, HandlerError},
    integrations::ports::{MediaServer, MediaServerError, RescanStore},
};

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
    /// did. A request made while the rescan runs, or while no media server is configured, stays pending.
    #[instrument(skip_all)]
    pub async fn run_due(&self, quiet: SignedDuration) -> Result<bool, RescanError> {
        let Some(requested_at) = self.store.requested_at().await? else { return Ok(false) };
        if self.clock.now().timestamp().duration_since(requested_at) < quiet {
            return Ok(false);
        }
        match self.server.refresh_library().await {
            Err(MediaServerError::NotConfigured) => return Ok(false),
            result => result?,
        }
        info!("media server rescanning");
        self.store.clear(requested_at).await?;
        Ok(true)
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

impl Rescans {
    async fn request(&self) -> Result<(), HandlerError> {
        self.store.request(self.clock.now().timestamp()).await?;
        debug!("media server rescan requested");
        Ok(())
    }
}

#[async_trait]
impl Handler<FilesImported> for Rescans {
    async fn handle(&self, _: &FilesImported) -> Result<(), HandlerError> {
        self.request().await
    }
}

#[async_trait]
impl Handler<FileRenamed> for Rescans {
    async fn handle(&self, _: &FileRenamed) -> Result<(), HandlerError> {
        self.request().await
    }
}

#[async_trait]
impl Handler<FileDeleted> for Rescans {
    async fn handle(&self, _: &FileDeleted) -> Result<(), HandlerError> {
        self.request().await
    }
}
