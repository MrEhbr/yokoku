use crate::ports::{ClientError, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{0} was already added")]
    AlreadyAdded(String),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
