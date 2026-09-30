use std::{path::Path, time::Duration};

use sqlx::{
    SqlitePool,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::db::DbError;

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens or creates the database file and its folder, and applies pending migrations.
    pub async fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let options =
            SqliteConnectOptions::new().filename(path).create_if_missing(true).journal_mode(SqliteJournalMode::Wal);
        Self::connect(options, SqlitePoolOptions::new()).await
    }

    /// A private in-memory database on a single connection.
    pub async fn open_in_memory() -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new().max_connections(1).idle_timeout(None).max_lifetime(None);
        Self::connect(SqliteConnectOptions::new().in_memory(true), pool).await
    }

    async fn connect(options: SqliteConnectOptions, pool: SqlitePoolOptions) -> Result<Self, DbError> {
        let options = options.foreign_keys(true).busy_timeout(Duration::from_secs(5));
        let pool = pool.connect_with(options).await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// The revision an upsert returned; `Conflict` unless it is one past `loaded`, the revision the
/// aggregate was loaded at.
pub(crate) fn saved_revision(returned: Option<u64>, loaded: u64) -> Result<u64, DbError> {
    returned.filter(|&returned| returned == loaded + 1).ok_or(DbError::Conflict)
}
