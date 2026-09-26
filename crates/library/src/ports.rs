use std::error::Error;

use async_trait::async_trait;
use yokoku_domain::{
    ExternalId, MediaKind, Movie, MovieId, MovieMetadata, Series, SeriesId, SeriesMetadata, StorageError,
};
use yokoku_events::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub kind: MediaKind,
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
}

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MetadataError>;
    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError>;
    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("{0} was not found at the metadata source")]
    NotFound(ExternalId),
    #[error("metadata source unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
}

/// Writes store the aggregate and `events` in one transaction. A save inserts an aggregate at
/// revision 0 and otherwise updates it only when the stored revision matches, then bumps
/// `revision`; a save made from an older revision, or of a removed aggregate, fails with
/// `StorageError::Conflict`.
#[async_trait]
pub trait SeriesRepo: Send + Sync {
    async fn get(&self, id: SeriesId) -> Result<Option<Series>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Series>, StorageError>;
    async fn ids(&self) -> Result<Vec<SeriesId>, StorageError>;
    /// Saves the series with all its seasons and episodes.
    async fn save(&self, series: &mut Series, events: &[Event]) -> Result<(), StorageError>;
    async fn remove(&self, id: SeriesId, events: &[Event]) -> Result<(), StorageError>;
}

/// Writes store the aggregate and `events` in one transaction. A save inserts an aggregate at
/// revision 0 and otherwise updates it only when the stored revision matches, then bumps
/// `revision`; a save made from an older revision, or of a removed aggregate, fails with
/// `StorageError::Conflict`.
#[async_trait]
pub trait MovieRepo: Send + Sync {
    async fn get(&self, id: MovieId) -> Result<Option<Movie>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Movie>, StorageError>;
    async fn ids(&self) -> Result<Vec<MovieId>, StorageError>;
    async fn save(&self, movie: &mut Movie, events: &[Event]) -> Result<(), StorageError>;
    async fn remove(&self, id: MovieId, events: &[Event]) -> Result<(), StorageError>;
}
