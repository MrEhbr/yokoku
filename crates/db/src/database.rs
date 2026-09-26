use std::{path::Path, time::Duration};

use sqlx::{
    Sqlite, SqlitePool, Transaction,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use yokoku_events::{Event, NewEvents};

use crate::{DbError, SqliteEventLog, event_log};

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
        SqliteEventLog::new(self.pool.clone())
    }

    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>, DbError> {
        Ok(self.pool.begin().await?)
    }

    /// Appends `events` inside `tx`, commits, then wakes event deliveries.
    pub async fn commit(&self, mut tx: Transaction<'static, Sqlite>, events: &[Event]) -> Result<(), DbError> {
        event_log::append(&mut tx, events).await?;
        tx.commit().await?;
        if !events.is_empty() {
            self.new_events.notify();
        }
        Ok(())
    }
}
