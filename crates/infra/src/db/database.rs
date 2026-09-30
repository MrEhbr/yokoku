use std::{path::Path, time::Duration};

use sqlx::{
    AssertSqlSafe, Sqlite, SqlitePool, Transaction,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::db::{DbError, codec::Int};

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

    /// Begins a transaction that bumps the row's revision; `Conflict` when the stored revision is
    /// not `revision`. A zero revision is a new row and bumps nothing.
    pub(crate) async fn begin_save(
        &self,
        table: &'static str,
        id: &str,
        revision: u64,
    ) -> Result<Transaction<'static, Sqlite>, DbError> {
        let mut tx = self.pool.begin().await?;
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
