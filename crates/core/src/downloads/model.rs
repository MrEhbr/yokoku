use std::path::PathBuf;

use jiff::Timestamp;
use yokoku_domain::{DownloadId, ItemId};

/// A torrent Yokoku tracks in the download client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub id: DownloadId,
    /// Lowercase hex info hash.
    pub hash: String,
    pub name: String,
    pub item: Option<ItemId>,
    /// For a series: the chosen season, used for duplicate checks and files whose names give none.
    /// `None` means no season was specified; the torrent's actual coverage is unknown.
    pub season: Option<u16>,
    pub status: TorrentStatus,
    pub added_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    /// When files from it reached the library.
    pub imported_at: Option<Timestamp>,
    /// Saves so far; storage refuses a save made from an older revision.
    pub revision: u64,
}

/// The client's view of a torrent at the last sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentStatus {
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

yokoku_domain::string_enum!(DownloadState, "download state" {
    Queued => "queued",
    Checking => "checking",
    Downloading => "downloading",
    Seeding => "seeding",
    Stopped => "stopped",
    Removed => "removed",
});

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

    pub(crate) fn mark_removed(&mut self) {
        self.status =
            TorrentStatus { state: DownloadState::Removed, download_rate: 0, eta: None, ..self.status.clone() };
    }
}

impl TorrentStatus {
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
