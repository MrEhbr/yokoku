use std::{error::Error, path::PathBuf};

use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_domain::{EpisodeSpan, ExternalId, ExternalIds, ItemId, MediaFileId, Movie, Rating, Series, StorageError};

#[async_trait]
pub trait MediaServer: Send + Sync {
    /// The server's name and version; fails when it cannot be reached.
    async fn version(&self) -> Result<String, MediaServerError>;

    /// Asks the server to scan its libraries for changes.
    async fn refresh_library(&self) -> Result<(), MediaServerError>;

    /// The episode files and movies the configured user has played.
    async fn played(&self) -> Result<Vec<Played>, MediaServerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MediaServerError {
    #[error("media server unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    #[error("media server refused the request: {0}")]
    Refused(String),
    #[error("no media server configured")]
    NotConfigured,
}

/// A file the media server's user has played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    /// As the media server sees it.
    pub path: PathBuf,
    /// `None` when the media server cannot place it.
    pub item: Option<PlayedItem>,
    pub at: Option<Timestamp>,
}

/// What a played file holds, with the ids the media server knows its series or movie by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayedItem {
    Episodes { series: Vec<ExternalId>, span: EpisodeSpan },
    Movie(Vec<ExternalId>),
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

/// A library file the media server's user has played, last at `at` when known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watched {
    pub file: MediaFileId,
    pub at: Option<Timestamp>,
}

#[async_trait]
pub trait WatchedStore: Send + Sync {
    /// Ordered by file id.
    async fn watched(&self) -> Result<Vec<Watched>, StorageError>;
    /// Replaces every watched file; a file no longer stored is skipped.
    async fn replace_watched(&self, watched: &[Watched]) -> Result<(), StorageError>;
}

/// The ids a ratings provider may look an item up by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatedItem {
    pub item: ItemId,
    pub source: ExternalId,
    pub external_ids: ExternalIds,
}

#[async_trait]
pub trait RatingsProvider: Send + Sync {
    /// The ratings it has for `items`; an item it does not know is left out.
    async fn ratings(&self, items: &[RatedItem]) -> Result<Vec<(ItemId, Rating)>, RatingsError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RatingsError {
    #[error("ratings unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    #[error("ratings are malformed")]
    Invalid(#[source] Box<dyn Error + Send + Sync>),
}

#[async_trait]
pub trait RatingsStore: Send + Sync {
    async fn ratings(&self, item: ItemId) -> Result<Vec<Rating>, StorageError>;
    /// Saves each rating over the item's stored one from the same source; skips an item no longer stored.
    async fn save_ratings(&self, ratings: &[(ItemId, Rating)]) -> Result<(), StorageError>;
}

impl From<&Series> for RatedItem {
    fn from(series: &Series) -> Self {
        Self { item: ItemId::Series(series.id), source: series.source, external_ids: series.external_ids.clone() }
    }
}

impl From<&Movie> for RatedItem {
    fn from(movie: &Movie) -> Self {
        Self { item: ItemId::Movie(movie.id), source: movie.source, external_ids: movie.external_ids.clone() }
    }
}
