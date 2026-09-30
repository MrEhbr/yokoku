use std::{
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use yokoku_domain::{DownloadId, FileTarget, ImportId, MediaFileId, Movie, MovieId, Series, SeriesId, StorageError};

use crate::media::{Import, ImportStatus, MediaFile, MediaInfo, RootFolder, detect::ListedFile};

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn is_dir(&self, path: &Path) -> Result<bool, FsError>;

    /// Names of the folders directly in `dir`, sorted; empty when `dir` does not exist. Hidden
    /// folders and names that are not UTF-8 are skipped.
    async fn folders(&self, dir: &Path) -> Result<Vec<String>, FsError>;

    /// Regular files under `dir` at any depth, ordered by path. Hidden entries, symlinked
    /// folders and names that are not UTF-8 are skipped.
    async fn files(&self, dir: &Path) -> Result<Vec<ListedFile>, FsError>;

    /// Files directly in `dir`, with the same rules as `files`.
    async fn files_in(&self, dir: &Path) -> Result<Vec<ListedFile>, FsError>;

    /// Moves a file, creating missing folders. Fails when `to` is another existing file.
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Removes `dir` and then each parent while they are empty, stopping before `stop`.
    async fn remove_empty_folders(&self, dir: &Path, stop: &Path) -> Result<(), FsError>;

    /// `None` when nothing is at `path`.
    async fn stat(&self, path: &Path) -> Result<Option<FileStat>, FsError>;

    /// Both files hold the same bytes.
    async fn same_contents(&self, a: &Path, b: &Path) -> Result<bool, FsError>;

    /// Links `to` to the same data as `from`, creating missing folders. Fails when `to` exists, and
    /// with `io::ErrorKind::CrossesDevices` when the two are on different file systems.
    async fn hard_link(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Copies through a hidden partial file, so `to` appears only once complete. Creates missing
    /// folders; fails when `to` exists.
    async fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// A file that is already gone counts as removed.
    async fn remove_file(&self, path: &Path) -> Result<(), FsError>;
}

/// Reads the streams of video files.
#[async_trait]
pub trait MediaProbe: Send + Sync {
    async fn probe(&self, path: &Path) -> Result<MediaInfo, ProbeError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("the media probe is not installed")]
    Missing,
    #[error("cannot probe {}: {reason}", path.display())]
    Failed { path: PathBuf, reason: String },
}

/// Exclusive right to change library files, shared by every process using the library.
#[async_trait]
pub trait LibraryLock: Send + Sync {
    /// Waits until no one else holds the lock; it is released when the guard drops.
    async fn acquire(&self) -> Result<LockGuard, FsError>;
}

pub struct LockGuard {
    _held: Box<dyn Send + Sync>,
}

impl LockGuard {
    pub fn new(held: impl Send + Sync + 'static) -> Self {
        Self { _held: Box::new(held) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStat {
    pub size: u64,
    pub device: u64,
    pub inode: u64,
}

impl FileStat {
    /// Both name the same data, as hard links do.
    pub fn same_file(&self, other: &FileStat) -> bool {
        (self.device, self.inode) == (other.device, other.inode)
    }
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
    async fn import_for_download(&self, download: DownloadId) -> Result<Option<Import>, StorageError>;
    /// Paths of the rows of imports not yet done, and of skipped rows of done ones.
    async fn claimed_paths(&self) -> Result<Vec<PathBuf>, StorageError>;
    /// Moves the oldest `Approved` import to `Importing` and returns it; one caller wins each import.
    async fn claim_next_approved(&self) -> Result<Option<Import>, StorageError>;
    /// Moves every `Importing` import back to `Approved`; returns how many.
    async fn reset_importing(&self) -> Result<u64, StorageError>;

    async fn media_info(&self, file: MediaFileId) -> Result<Option<MediaInfo>, StorageError>;
    /// Replaces the file's details; a file no longer stored is left out.
    async fn save_media_info(&self, file: MediaFileId, info: &MediaInfo) -> Result<(), StorageError>;
    /// Files never probed, ordered by path.
    async fn files_without_media_info(&self) -> Result<Vec<MediaFile>, StorageError>;

    /// Stores `changes` in one transaction.
    async fn save(&self, changes: &Changes) -> Result<(), StorageError>;
}

#[derive(Debug, Default)]
pub struct Changes {
    pub added_files: Vec<MediaFile>,
    pub removed_files: Vec<MediaFileId>,
    /// New paths of files that moved.
    pub renamed_files: Vec<(MediaFileId, PathBuf)>,
    /// New targets of files whose episodes were renumbered.
    pub retargeted_files: Vec<(MediaFileId, FileTarget)>,
    /// Inserted or replaced with all their rows.
    pub imports: Vec<Import>,
}
