use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use yokoku_core::integrations::ports::RatingsStore;
use yokoku_domain::{ItemId, Rating, StorageError};

use crate::db::{Database, DbError};

/// A rating in an item's `ratings` column, keyed by its source.
#[derive(Serialize, Deserialize)]
struct StoredRating {
    value: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    votes: Option<u32>,
}

#[async_trait]
impl RatingsStore for Database {
    async fn ratings(&self, item: ItemId) -> Result<Vec<Rating>, StorageError> {
        Ok(self.load_ratings(item).await?)
    }

    async fn save_ratings(&self, ratings: &[(ItemId, Rating)]) -> Result<(), StorageError> {
        Ok(self.store_ratings(ratings).await?)
    }
}

impl Database {
    /// Skips a source this version doesn't know.
    async fn load_ratings(&self, item: ItemId) -> Result<Vec<Rating>, DbError> {
        let query = match item {
            ItemId::Series(id) => sqlx::query_scalar("SELECT ratings FROM series WHERE id = ?").bind(id.to_string()),
            ItemId::Movie(id) => sqlx::query_scalar("SELECT ratings FROM movies WHERE id = ?").bind(id.to_string()),
        };
        let stored: Option<Json<BTreeMap<String, StoredRating>>> = query.fetch_optional(self.pool()).await?;
        Ok(stored
            .map(|Json(stored)| stored)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(source, stored)| {
                Some(Rating { source: source.parse().ok()?, value: stored.value, votes: stored.votes })
            })
            .collect())
    }

    async fn store_ratings(&self, ratings: &[(ItemId, Rating)]) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        for &(item, rating) in ratings {
            let (sql, id) = match item {
                ItemId::Series(id) => {
                    ("UPDATE series SET ratings = json_set(ratings, '$.' || ?, json(?)) WHERE id = ?", id.to_string())
                },
                ItemId::Movie(id) => {
                    ("UPDATE movies SET ratings = json_set(ratings, '$.' || ?, json(?)) WHERE id = ?", id.to_string())
                },
            };
            let stored = StoredRating { value: rating.value, votes: rating.votes };
            sqlx::query(sql).bind(rating.source.as_str()).bind(Json(stored)).bind(id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
