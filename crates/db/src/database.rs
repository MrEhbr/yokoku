use std::{path::Path, time::Duration};

use sqlx::{
    AssertSqlSafe, Sqlite, SqlitePool, Transaction,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use yokoku_events::NewEvents;

use crate::{DbError, SqliteEventLog, codec::Int};

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    new_events: NewEvents,
}

impl Database {
    /// Opens or creates the database file and applies pending migrations.
    pub async fn open(path: &Path) -> Result<Self, DbError> {
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
        Ok(Self { pool, new_events: NewEvents::new() })
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn new_events(&self) -> &NewEvents {
        &self.new_events
    }

    pub fn event_log(&self) -> SqliteEventLog {
        SqliteEventLog::new(self.pool.clone(), self.new_events.clone())
    }

    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>, DbError> {
        Ok(self.pool.begin().await?)
    }

    /// Begins a transaction that bumps the row's revision; `Conflict` when the stored revision is
    /// not `revision`. A zero revision is a new row and bumps nothing.
    pub(crate) async fn begin_save(
        &self,
        table: &'static str,
        id: &str,
        revision: u64,
    ) -> Result<Transaction<'static, Sqlite>, DbError> {
        let mut tx = self.begin().await?;
        if revision > 0 {
            let claimed = sqlx::query(AssertSqlSafe(format!(
                "UPDATE {table} SET revision = revision + 1 WHERE id = ? AND revision = ?"
            )))
            .bind(id)
            .bind(Int(revision))
            .execute(&mut *tx)
            .await?;
            if claimed.rows_affected() == 0 {
                return Err(DbError::Conflict);
            }
        }
        Ok(tx)
    }
}
