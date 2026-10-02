use std::error::Error;

use async_trait::async_trait;
use yokoku_domain::{
    ArtworkKind, ExternalId, FileTarget, ItemFolder, ItemId, MediaFileId, MediaKind, Movie, MovieId, MovieMetadata,
    Series, SeriesId, SeriesMetadata, StorageError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub kind: MediaKind,
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    /// Empty when the source has none.
    pub overview: String,
}

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Movies and series, or only those of `kind`.
    async fn search(&self, query: &str, kind: Option<MediaKind>) -> Result<Vec<SearchResult>, MetadataError>;
    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError>;
    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError>;
}

/// Artwork images at the metadata sources.
#[async_trait]
pub trait ArtworkSource: Send + Sync {
    /// The image at `path` as the item's source stored it: a TMDB path or a TVDB URL.
    async fn fetch(&self, source: ExternalId, kind: ArtworkKind, path: &str) -> Result<Vec<u8>, MetadataError>;
    /// Like `fetch`, at a small size for previews where the source has one.
    async fn fetch_thumbnail(
        &self,
        source: ExternalId,
        kind: ArtworkKind,
        path: &str,
    ) -> Result<Vec<u8>, MetadataError>;
}

/// Artwork images kept on this host, at most one of each kind per item.
#[async_trait]
pub trait ArtworkCache: Send + Sync {
    async fn get(&self, item: ItemId, kind: ArtworkKind, name: &str) -> Result<Option<Vec<u8>>, StorageError>;
    /// Stores `image` as the item's `kind` image `name`, replacing its other image of that kind.
    async fn put(&self, item: ItemId, kind: ArtworkKind, name: &str, image: &[u8]) -> Result<(), StorageError>;
    /// Removes every image of the item.
    async fn remove(&self, item: ItemId) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("{0} was not found at the metadata source")]
    NotFound(ExternalId),
    #[error("metadata source unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    /// Credentials are missing or rejected; trying again does not help.
    #[error("metadata source refused the request: {0}")]
    Refused(String),
    #[error("unexpected answer from the metadata source")]
    Invalid(#[source] Box<dyn Error + Send + Sync>),
}

/// A save inserts an aggregate at revision 0 and otherwise updates it only when the stored
/// revision matches, then bumps `revision`; a save made from an older revision, or of a removed
/// aggregate, fails with `StorageError::Conflict`.
#[async_trait]
pub trait SeriesRepo: Send + Sync {
    async fn get(&self, id: SeriesId) -> Result<Option<Series>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Series>, StorageError>;
    async fn find_by_folder(&self, folder: &ItemFolder) -> Result<Option<SeriesId>, StorageError>;
    async fn ids(&self) -> Result<Vec<SeriesId>, StorageError>;
    /// Every series, in the order of `ids`.
    async fn all(&self) -> Result<Vec<Series>, StorageError>;
    /// Saves the series with all its seasons and episodes.
    async fn save(&self, series: &mut Series) -> Result<(), StorageError>;
    async fn remove(&self, id: SeriesId) -> Result<(), StorageError>;
}

/// Read-only view of media's library files.
#[async_trait]
pub trait MediaFiles: Send + Sync {
    /// `None` when the file is no longer in the library.
    async fn target(&self, file: MediaFileId) -> Result<Option<FileTarget>, StorageError>;
}

/// A save inserts an aggregate at revision 0 and otherwise updates it only when the stored
/// revision matches, then bumps `revision`; a save made from an older revision, or of a removed
/// aggregate, fails with `StorageError::Conflict`.
#[async_trait]
pub trait MovieRepo: Send + Sync {
    async fn get(&self, id: MovieId) -> Result<Option<Movie>, StorageError>;
    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Movie>, StorageError>;
    async fn find_by_folder(&self, folder: &ItemFolder) -> Result<Option<MovieId>, StorageError>;
    async fn ids(&self) -> Result<Vec<MovieId>, StorageError>;
    /// Every movie, in the order of `ids`.
    async fn all(&self) -> Result<Vec<Movie>, StorageError>;
    async fn save(&self, movie: &mut Movie) -> Result<(), StorageError>;
    async fn remove(&self, id: MovieId) -> Result<(), StorageError>;
}
