use yokoku_library::ports::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database query failed")]
    Query(#[from] sqlx::Error),
    #[error("database migration failed")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("event encoding failed")]
    EventEncoding(#[from] serde_json::Error),
    #[error("invalid stored value: {0}")]
    InvalidValue(String),
}

impl From<DbError> for StorageError {
    fn from(error: DbError) -> Self {
        Self::new(error)
    }
}
