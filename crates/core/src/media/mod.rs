//! Import pipeline, review, scan, rename, and delete.

mod deleter;
pub mod detect;
mod error;
mod files;
mod import;
mod model;
pub mod ports;
mod prober;
mod rename;
mod roots;
mod scan;

pub use deleter::Deleter;
pub use error::MediaError;
pub use import::{
    Approval, Destination, ImportMode, ImportPlanner, ImportReview, ImportSettings, Importer, Problem, ReviewRow,
    Reviewer,
};
pub use model::{
    AudioStream, Episodes, Import, ImportRow, ImportStatus, MediaFile, MediaInfo, Resolution, RootFolder, RootKind,
    RowMatch, SubtitleStream, VideoStream,
};
pub use prober::{FileDetails, ProbeReport, Prober};
pub use rename::{Move, Rename, RenameFailure, RenamePlan, RenameReport, RenameScope, Renamer, SkipReason, Skipped};
pub use roots::RootFolders;
pub use scan::{ScanReport, Scanner};

pub use crate::media::detect::Conflict;
