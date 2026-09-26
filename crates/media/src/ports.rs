use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use yokoku_detect::DownloadFile;
use yokoku_domain::{ImportId, MediaFileId, Movie, MovieId, Series, SeriesId};
use yokoku_events::Event;

use crate::{Import, ImportStatus, MediaFile, RootFolder};

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn is_dir(&self, path: &Path) -> Result<bool, FsError>;

    /// Regular files under `dir` at any depth, ordered by path. Hidden entries, symlinked
    /// folders and names that are not UTF-8 are skipped.
    async fn files(&self, dir: &Path) -> Result<Vec<DownloadFile>, FsError>;

    /// Files directly in `dir`, with the same rules as `files`.
    async fn files_in(&self, dir: &Path) -> Result<Vec<DownloadFile>, FsError>;

    /// Moves a file, creating missing folders. Fails when `to` is another existing file.
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Removes `dir` and then each parent while they are empty, stopping before `stop`.
    async fn remove_empty_folders(&self, dir: &Path, stop: &Path) -> Result<(), FsError>;
}

#[derive(Debug, thiserror::Error)]
#[error("{}: {source}", path.display())]
pub struct FsError {
    pub path: PathBuf,
    #[source]
    pub source: io::Error,
}

/// Read-only view of the library.
#[async_trait]
pub trait Catalog: Send + Sync {
    async fn all_series(&self) -> Result<Vec<Series>, StorageError>;
    async fn all_movies(&self) -> Result<Vec<Movie>, StorageError>;
    async fn series(&self, id: SeriesId) -> Result<Option<Series>, StorageError>;
    async fn movie(&self, id: MovieId) -> Result<Option<Movie>, StorageError>;
}

#[async_trait]
pub trait MediaRepo: Send + Sync {
    async fn root_folders(&self) -> Result<Vec<RootFolder>, StorageError>;
    async fn add_root_folder(&self, root: &RootFolder) -> Result<(), StorageError>;
    /// `false` when no root folder has this path.
    async fn remove_root_folder(&self, path: &Path) -> Result<bool, StorageError>;

    async fn files(&self) -> Result<Vec<MediaFile>, StorageError>;
    async fn import(&self, id: ImportId) -> Result<Option<Import>, StorageError>;
    /// Oldest first.
    async fn imports(&self, status: ImportStatus) -> Result<Vec<Import>, StorageError>;

    /// Stores `changes` and appends `events` in one transaction.
    async fn save(&self, changes: &Changes, events: &[Event]) -> Result<(), StorageError>;
}

#[derive(Debug, Default)]
pub struct Changes {
    pub added_files: Vec<MediaFile>,
    pub removed_files: Vec<MediaFileId>,
    /// Inserted or replaced with all their rows.
    pub imports: Vec<Import>,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct StorageError(Box<dyn Error + Send + Sync>);

impl StorageError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
