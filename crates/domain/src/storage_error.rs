use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("the item was changed or removed at the same time")]
    Conflict,
    #[error(transparent)]
    Other(Box<dyn Error + Send + Sync>),
}

impl StorageError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::Other(source.into())
    }
}
