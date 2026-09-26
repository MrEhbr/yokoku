use yokoku_domain::StorageError;

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
    #[error("the row was changed or removed at the same time")]
    Conflict,
}

impl From<DbError> for StorageError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::Conflict => Self::Conflict,
            error => Self::new(error),
        }
    }
}
