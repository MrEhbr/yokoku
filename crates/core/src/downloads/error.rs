use yokoku_domain::StorageError;

use crate::downloads::ports::{ClientError, IndexerError};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{0} was already added")]
    AlreadyAdded(String),
    #[error("the movie already has a file or an active torrent")]
    MovieAlreadyHasDownload,
    #[error("the series already has an unfinished torrent for this season")]
    SeriesSeasonAlreadyDownloading,
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    Indexer(#[from] IndexerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
