use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_integrations::ports::{RescanStore, StorageError};

use crate::{Database, DbError};

#[async_trait]
impl RescanStore for Database {
    async fn requested_at(&self) -> Result<Option<Timestamp>, StorageError> {
        let requested_at: Option<i64> = sqlx::query_scalar("SELECT requested_at FROM media_server_rescan")
            .fetch_optional(self.pool())
            .await
            .map_err(DbError::from)?;
        let timestamp = |millis: i64| {
            Timestamp::from_millisecond(millis).map_err(|_| DbError::InvalidValue(format!("rescan time {millis}")))
        };
        Ok(requested_at.map(timestamp).transpose()?)
    }

    async fn request(&self, at: Timestamp) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO media_server_rescan (id, requested_at) VALUES (1, ?)
             ON CONFLICT (id) DO UPDATE SET requested_at = max(requested_at, excluded.requested_at)",
        )
        .bind(at.as_millisecond())
        .execute(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(())
    }

    async fn clear(&self, at: Timestamp) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM media_server_rescan WHERE requested_at = ?")
            .bind(at.as_millisecond())
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(())
    }
}
