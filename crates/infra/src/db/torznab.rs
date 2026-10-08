use sqlx::types::Json;

use crate::{
    db::{Database, DbError},
    indexers::TorznabFeed,
};

impl Database {
    pub async fn torznab_feeds(&self) -> Result<Vec<TorznabFeed>, DbError> {
        let rows: Vec<(Json<TorznabFeed>,)> =
            sqlx::query_as("SELECT value FROM torznab_feeds ORDER BY id").fetch_all(self.pool()).await?;
        Ok(rows.into_iter().map(|(value,)| value.0).collect())
    }

    pub async fn save_torznab_feed(&self, feed: &TorznabFeed) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO torznab_feeds (id, value) VALUES (?, ?) ON CONFLICT (id) DO UPDATE SET value = excluded.value",
        )
        .bind(feed.id.as_str())
        .bind(Json(feed))
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn remove_torznab_feed(&self, id: &str) -> Result<bool, DbError> {
        Ok(sqlx::query("DELETE FROM torznab_feeds WHERE id = ?").bind(id).execute(self.pool()).await?.rows_affected()
            > 0)
    }
}
