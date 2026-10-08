use yokoku_domain::StorageError;

use crate::downloads::ports::{ClientError, IndexerError};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{0} was already added")]
    AlreadyAdded(String),
    #[error("the movie already has a file or an active torrent")]
    MovieAlreadyHasDownload,
    #[error("every known episode of this season already has a library file")]
    SeriesSeasonAlreadyDownloaded,
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    Indexer(#[from] IndexerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
