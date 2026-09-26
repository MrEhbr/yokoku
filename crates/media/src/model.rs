use std::path::PathBuf;

use jiff::Timestamp;
use yokoku_domain::{Confidence, FileTarget, ImportId, MediaFileId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    Series,
    Movies,
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

/// Files waiting for the user to confirm what they hold (FR-4.11, FR-8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub id: ImportId,
    /// The folder or file the rows come from.
    pub source: PathBuf,
    pub status: ImportStatus,
    /// Ordered by path.
    pub rows: Vec<ImportRow>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStatus {
    NeedsReview,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRow {
    pub path: PathBuf,
    pub size: u64,
    pub target: Option<FileTarget>,
    pub confidence: Confidence,
    pub skipped: bool,
}
