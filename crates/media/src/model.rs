use std::{path::PathBuf, time::Duration};

use jiff::Timestamp;
use yokoku_domain::{Confidence, DownloadId, FileTarget, ImportId, MediaFileId};
use yokoku_events::LinkedFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    Series,
    Movies,
}

yokoku_domain::string_enum!(RootKind, "root folder kind" {
    Series => "series",
    Movies => "movies",
});

impl From<FileTarget> for RootKind {
    fn from(target: FileTarget) -> Self {
        match target {
            FileTarget::Episodes { .. } => Self::Series,
            FileTarget::Movie(_) => Self::Movies,
        }
    }
}

/// A folder holding the library's series or movies (FR-8.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootFolder {
    pub kind: RootKind,
    pub path: PathBuf,
}

/// A video file in a root folder, linked to what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaFile {
    pub id: MediaFileId,
    pub path: PathBuf,
    pub size: u64,
    pub target: FileTarget,
    pub added_at: Timestamp,
}

impl MediaFile {
    pub(crate) fn linked(&self) -> LinkedFile {
        LinkedFile { file: self.id, path: self.path.clone(), target: self.target }
    }
}

/// Files waiting for the user to confirm what they hold (FR-4.11, FR-8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub id: ImportId,
    /// The folder or file the rows come from.
    pub source: PathBuf,
    /// The download the files come from; files found by a scan have none and stay where they are.
    pub download: Option<DownloadId>,
    pub status: ImportStatus,
    /// Why the last attempt failed.
    pub error: Option<String>,
    /// Ordered by path.
    pub rows: Vec<ImportRow>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStatus {
    NeedsReview,
    /// Waiting to be carried out.
    Approved,
    Importing,
    Done,
    Failed,
}

yokoku_domain::string_enum!(ImportStatus, "import status" {
    NeedsReview => "needs_review",
    Approved => "approved",
    Importing => "importing",
    Done => "done",
    Failed => "failed",
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRow {
    pub path: PathBuf,
    pub size: u64,
    pub target: Option<FileTarget>,
    pub confidence: Confidence,
    pub skipped: bool,
    /// Replaces the library file that already holds the target.
    pub replace: bool,
}

/// Streams a probe read from a video file (FR-8.6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub video: Option<VideoStream>,
    /// In file order.
    pub audio: Vec<AudioStream>,
    /// Subtitles inside the file, in file order.
    pub subtitles: Vec<SubtitleStream>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStream {
    pub codec: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStream {
    pub codec: String,
    /// ISO 639-2, like `eng`.
    pub language: Option<String>,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleStream {
    pub codec: String,
    /// ISO 639-2, like `eng`.
    pub language: Option<String>,
    pub forced: bool,
}
