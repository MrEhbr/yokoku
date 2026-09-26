use std::path::PathBuf;

use yokoku_domain::{EpisodeSpan, ImportId, MovieId, SeriesId, StorageError};
use yokoku_naming::NamingError;

use crate::{
    RootKind,
    ports::{FsError, ProbeError},
};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("{} is not an absolute path", .0.display())]
    RelativePath(PathBuf),
    #[error("{} is not a folder", .0.display())]
    NotAFolder(PathBuf),
    #[error("{} overlaps root folder {}", .path.display(), .existing.display())]
    OverlappingRoot { path: PathBuf, existing: PathBuf },
    #[error("{} is not a root folder", .0.display())]
    RootNotFound(PathBuf),
    #[error("{} is a {} root folder", .path.display(), .kind.as_str())]
    WrongRootKind { path: PathBuf, kind: RootKind },
    #[error("{} still holds {items} library items", .path.display())]
    RootInUse { path: PathBuf, items: usize },
    #[error("import {0} does not exist")]
    ImportNotFound(ImportId),
    #[error("import {0} is not waiting for review")]
    NotInReview(ImportId),
    #[error("only files from a download can replace library files")]
    ReplaceInPlace,
    #[error("import {0} has not failed")]
    NotFailed(ImportId),
    #[error("no library file holds it")]
    NoFile,
    #[error("import has no row {0}")]
    RowNotFound(usize),
    #[error("series {0} is not in the library")]
    SeriesNotFound(SeriesId),
    #[error("movie {0} is not in the library")]
    MovieNotFound(MovieId),
    #[error("{0} is not in the series")]
    EpisodesNotFound(EpisodeSpan),
    #[error("rows {} have no match; match or skip them", numbers(.0))]
    UnmatchedRows(Vec<usize>),
    #[error("rows {} conflict with each other or with library files; match or skip them", numbers(.0))]
    ConflictingRows(Vec<usize>),
    #[error("row {0} has no match")]
    RowUnmatched(usize),
    #[error("{} already exists", .0.display())]
    AlreadyExists(PathBuf),
    #[error("{} is missing", .0.display())]
    SourceMissing(PathBuf),
    #[error(transparent)]
    Naming(#[from] NamingError),
    #[error(transparent)]
    FileSystem(#[from] FsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Probe(#[from] ProbeError),
}

fn numbers(rows: &[usize]) -> String {
    rows.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}
