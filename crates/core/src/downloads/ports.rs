use std::error::Error;

use async_trait::async_trait;
use yokoku_domain::{DownloadId, StorageError};

use crate::downloads::{Download, TorrentStatus};

/// The label `DownloadClient::add` puts on every torrent it adds.
pub const LABEL: &str = "yokoku";

#[async_trait]
pub trait DownloadClient: Send + Sync {
    /// The client's name and version; fails when it cannot be reached.
    async fn version(&self) -> Result<String, ClientError>;

    /// Adds the torrent labelled `LABEL`.
    async fn add(&self, torrent: &TorrentSource) -> Result<AddedTorrent, ClientError>;

    /// The torrents with these info hashes; ones the client no longer has are left out.
    async fn torrents(&self, hashes: &[String]) -> Result<Vec<Torrent>, ClientError>;

    /// Every torrent the client has.
    async fn all_torrents(&self) -> Result<Vec<Torrent>, ClientError>;

    /// Removes a torrent, and its downloaded files with `delete_data`; an unknown hash is no error.
    async fn remove(&self, hash: &str, delete_data: bool) -> Result<(), ClientError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorrentSource {
    Magnet(String),
    /// The contents of a .torrent file.
    File(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedTorrent {
    /// Lowercase hex info hash.
    pub hash: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Torrent {
    /// Lowercase hex info hash.
    pub hash: String,
    pub name: String,
    pub status: TorrentStatus,
    /// Every selected byte is downloaded and verified.
    pub complete: bool,
    /// Seeding reached the client's ratio or idle limit.
    pub seeding_done: bool,
    pub labels: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("download client unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    #[error("download client refused the request: {0}")]
    Refused(String),
}

/// A save inserts a download at revision 0 and otherwise updates it only when the stored revision
/// matches, then bumps `revision`; a save made from an older revision, or a new download whose hash
/// is already stored, fails with `StorageError::Conflict`.
#[async_trait]
pub trait DownloadRepo: Send + Sync {
    async fn get(&self, id: DownloadId) -> Result<Option<Download>, StorageError>;
    async fn find_by_hash(&self, hash: &str) -> Result<Option<Download>, StorageError>;
    /// Newest first.
    async fn list(&self) -> Result<Vec<Download>, StorageError>;
    async fn save(&self, download: &mut Download) -> Result<(), StorageError>;
}
