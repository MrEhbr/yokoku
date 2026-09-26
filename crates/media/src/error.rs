use std::path::PathBuf;

use yokoku_domain::{EpisodeSpan, ImportId, MovieId, SeriesId};

use crate::ports::{FsError, StorageError};

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
    #[error("import {0} does not exist")]
    ImportNotFound(ImportId),
    #[error("import {0} is already done")]
    ImportDone(ImportId),
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
    #[error(transparent)]
    FileSystem(#[from] FsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

fn numbers(rows: &[usize]) -> String {
    rows.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}
