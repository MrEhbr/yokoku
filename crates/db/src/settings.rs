use serde_json::Value;

use crate::{Database, DbError};

impl Database {
    /// Stored settings by dotted key, ordered by key.
    pub async fn settings(&self) -> Result<Vec<(String, Value)>, DbError> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT key, value FROM settings ORDER BY key").fetch_all(self.pool()).await?;
        rows.into_iter().map(|(key, value)| Ok((key, serde_json::from_str(&value)?))).collect()
    }

    pub async fn set_setting(&self, key: &str, value: &Value) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// `false` when nothing was stored under `key`.
    pub async fn remove_setting(&self, key: &str) -> Result<bool, DbError> {
        let removed = sqlx::query("DELETE FROM settings WHERE key = ?").bind(key).execute(self.pool()).await?;
        Ok(removed.rows_affected() > 0)
    }
}
