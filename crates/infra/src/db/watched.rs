use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_core::integrations::ports::{Watched, WatchedStore};
use yokoku_domain::{MediaFileId, StorageError};

use crate::db::{Database, DbError, codec::Text};

#[derive(sqlx::FromRow)]
struct WatchedRow {
    file_id: Text<MediaFileId>,
    watched_at: Option<Text<Timestamp>>,
}

#[async_trait]
impl WatchedStore for Database {
    async fn watched(&self) -> Result<Vec<Watched>, StorageError> {
        let rows: Vec<WatchedRow> = sqlx::query_as("SELECT file_id, watched_at FROM watched_files ORDER BY file_id")
            .fetch_all(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(rows.into_iter().map(|row| Watched { file: row.file_id.0, at: row.watched_at.map(|at| at.0) }).collect())
    }

    async fn replace_watched(&self, watched: &[Watched]) -> Result<(), StorageError> {
        Ok(self.store_watched(watched).await?)
    }
}

impl Database {
    async fn store_watched(&self, watched: &[Watched]) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM watched_files").execute(&mut *tx).await?;
        for watched in watched {
            sqlx::query("INSERT INTO watched_files (file_id, watched_at) SELECT id, ? FROM media_files WHERE id = ?")
                .bind(watched.at.map(Text))
                .bind(watched.file.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
