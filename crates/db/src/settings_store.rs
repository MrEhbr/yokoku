use async_trait::async_trait;
use serde_json::Value;
use sqlx::types::Json;
use yokoku_domain::{SettingsStore, StorageError};

use crate::{Database, DbError};

#[async_trait]
impl SettingsStore for Database {
    async fn settings(&self) -> Result<Vec<(String, Value)>, StorageError> {
        let rows: Vec<(String, Json<Value>)> = sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
            .fetch_all(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(rows.into_iter().map(|(key, value)| (key, value.0)).collect())
    }

    async fn set_setting(&self, key: &str, value: &Value) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(Json(value))
        .execute(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(())
    }

    async fn remove_setting(&self, key: &str) -> Result<bool, StorageError> {
        let removed = sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(removed.rows_affected() > 0)
    }
}
