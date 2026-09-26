use std::error::Error;

use async_trait::async_trait;
use jiff::Zoned;
use yokoku_domain::{ExternalId, Movie, MovieId, MovieMetadata, Series, SeriesId, SeriesMetadata};
use yokoku_events::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Series,
    Movie,
}

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

pub trait Clock: Send + Sync {
    /// The current time in the user's time zone.
    fn now(&self) -> Zoned;
}

/// Writes store the aggregate and `events` in one transaction.
#[async_trait]
pub trait SeriesRepo: Send + Sync {
    async fn get(&self, id: SeriesId) -> Result<Option<Series>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Series>, StorageError>;
    async fn ids(&self) -> Result<Vec<SeriesId>, StorageError>;
    /// Inserts or updates the series with all its seasons and episodes.
    async fn save(&self, series: &Series, events: &[Event]) -> Result<(), StorageError>;
    async fn remove(&self, id: SeriesId, events: &[Event]) -> Result<(), StorageError>;
}

/// Writes store the aggregate and `events` in one transaction.
#[async_trait]
pub trait MovieRepo: Send + Sync {
    async fn get(&self, id: MovieId) -> Result<Option<Movie>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Movie>, StorageError>;
    async fn ids(&self) -> Result<Vec<MovieId>, StorageError>;
    /// Inserts or updates the movie.
    async fn save(&self, movie: &Movie, events: &[Event]) -> Result<(), StorageError>;
    async fn remove(&self, id: MovieId, events: &[Event]) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct StorageError(Box<dyn Error + Send + Sync>);

impl StorageError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
