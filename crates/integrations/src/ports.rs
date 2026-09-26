use std::error::Error;

use async_trait::async_trait;
use jiff::Timestamp;

#[async_trait]
pub trait MediaServer: Send + Sync {
    /// The server's name and version; fails when it cannot be reached.
    async fn version(&self) -> Result<String, MediaServerError>;

    /// Asks the server to scan its libraries for changes.
    async fn refresh_library(&self) -> Result<(), MediaServerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MediaServerError {
    #[error("media server unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    #[error("media server refused the request: {0}")]
    Refused(String),
}

/// The one pending rescan request.
#[async_trait]
pub trait RescanStore: Send + Sync {
    async fn requested_at(&self) -> Result<Option<Timestamp>, StorageError>;
    /// Keeps the later of the pending request and `at`.
    async fn request(&self, at: Timestamp) -> Result<(), StorageError>;
    /// Clears the request only if it is still the one made at `at`.
    async fn clear(&self, at: Timestamp) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct StorageError(Box<dyn Error + Send + Sync>);

impl StorageError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
