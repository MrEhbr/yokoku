use std::path::PathBuf;

use jiff::Timestamp;
use yokoku_domain::{DownloadId, ItemId};

/// A torrent Yokoku added to the download client (FR-3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub id: DownloadId,
    /// Lowercase hex info hash.
    pub hash: String,
    pub name: String,
    pub item: Option<ItemId>,
    pub status: DownloadStatus,
    pub added_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    /// Saves so far; storage refuses a save made from an older revision.
    pub revision: u64,
}

/// The client's view of a torrent at the last sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadStatus {
    pub state: DownloadState,
    /// Bytes selected for download.
    pub size: u64,
    /// Bytes of `size` already downloaded.
    pub done: u64,
    /// Bytes per second.
    pub download_rate: u64,
    /// Seconds until done, when the client can tell.
    pub eta: Option<u64>,
    pub download_dir: PathBuf,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Checking,
    Downloading,
    Seeding,
    Stopped,
    /// No longer in the client.
    Removed,
}

impl Download {
    /// Where the client puts the torrent's file or folder.
    pub fn content_path(&self) -> PathBuf {
        self.status.download_dir.join(&self.name)
    }

    /// 0 to 100, rounded down.
    pub fn percent_done(&self) -> u8 {
        match self.status.size {
            0 => 0,
            size => u8::try_from(self.status.done.min(size) * 100 / size).unwrap_or(100),
        }
    }
}

impl DownloadStatus {
    /// Before the client has reported anything.
    pub fn unknown() -> Self {
        Self {
            state: DownloadState::Queued,
            size: 0,
            done: 0,
            download_rate: 0,
            eta: None,
            download_dir: PathBuf::new(),
            error: None,
        }
    }
}
