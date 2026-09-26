use std::error::Error;

use async_trait::async_trait;
use yokoku_domain::{ExternalId, Movie, MovieId, Series, SeriesId};
use yokoku_events::Event;

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
