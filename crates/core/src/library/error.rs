use std::path::PathBuf;

use yokoku_domain::{EpisodeRef, ExternalId, InvalidFolderName, MovieId, SeriesId, StorageError};

use crate::library::ports::MetadataError;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("{0} is already in the library")]
    AlreadyInLibrary(ExternalId),
    #[error(transparent)]
    InvalidFolder(#[from] InvalidFolderName),
    #[error("{} already belongs to another item", .0.display())]
    FolderTaken(PathBuf),
    #[error("series {0} is not in the library")]
    SeriesNotFound(SeriesId),
    #[error("movie {0} is not in the library")]
    MovieNotFound(MovieId),
    #[error("season {0} does not exist")]
    SeasonNotFound(u16),
    #[error("episode S{:02}E{:02} does not exist", .0.season, .0.episode)]
    EpisodeNotFound(EpisodeRef),
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
