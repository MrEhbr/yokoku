use async_trait::async_trait;
use sqlx::types::Json;
use yokoku_domain::StorageError;
use yokoku_events::{Correlated, DeliveryFailure, Event, EventId, EventStore, Recorded};

use crate::db::Database;

const UPSERT_POSITION: &str = "
    INSERT INTO subscriber_positions (subscriber, last_event_id) VALUES (?, ?)
    ON CONFLICT (subscriber) DO UPDATE SET last_event_id = excluded.last_event_id";

/// The SQLite tables `events`, `subscriber_positions` and `failed_deliveries`.
#[async_trait]
impl EventStore for Database {
    async fn append(&self, events: &[Correlated]) -> Result<(), StorageError> {
        let mut tx = self.pool().begin().await.map_err(StorageError::new)?;
        for Correlated { correlation, event } in events {
            sqlx::query("INSERT INTO events (payload, correlation) VALUES (?, ?)")
                .bind(Json(event))
                .bind(correlation.to_string())
                .execute(&mut *tx)
                .await
                .map_err(StorageError::new)?;
        }
        tx.commit().await.map_err(StorageError::new)
    }

    async fn last_delivered(&self, subscriber: &str) -> Result<Option<EventId>, StorageError> {
        let position: Option<i64> =
            sqlx::query_scalar("SELECT last_event_id FROM subscriber_positions WHERE subscriber = ?")
                .bind(subscriber)
                .fetch_optional(self.pool())
                .await
                .map_err(StorageError::new)?;

        Ok(position.map(EventId))
    }

    async fn read_after(&self, after: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError> {
        let rows: Vec<EventRow> =
            sqlx::query_as("SELECT id, payload, occurred_at, correlation FROM events WHERE id > ? ORDER BY id LIMIT ?")
                .bind(after.map_or(0, |id| id.0))
                .bind(limit)
                .fetch_all(self.pool())
                .await
                .map_err(StorageError::new)?;

        rows.into_iter().map(Recorded::try_from).collect()
    }

    async fn read_before(&self, before: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, StorageError> {
        let rows: Vec<EventRow> = sqlx::query_as(
            "SELECT id, payload, occurred_at, correlation FROM events WHERE id < ? ORDER BY id DESC LIMIT ?",
        )
        .bind(before.map_or(i64::MAX, |id| id.0))
        .bind(limit)
        .fetch_all(self.pool())
        .await
        .map_err(StorageError::new)?;

        rows.into_iter().map(Recorded::try_from).collect()
    }

    async fn mark_delivered(&self, subscriber: &str, event: EventId) -> Result<(), StorageError> {
        sqlx::query(UPSERT_POSITION)
            .bind(subscriber)
            .bind(event.0)
            .execute(self.pool())
            .await
            .map_err(StorageError::new)?;

        Ok(())
    }

    async fn give_up(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError> {
        let mut tx = self.pool().begin().await.map_err(StorageError::new)?;

        sqlx::query(
            "INSERT INTO failed_deliveries (subscriber, event_id, error, attempts) VALUES (?, ?, ?, ?)
             ON CONFLICT (subscriber, event_id) DO UPDATE
             SET error = excluded.error, attempts = excluded.attempts, failed_at = excluded.failed_at",
        )
        .bind(subscriber)
        .bind(failure.event.0)
        .bind(&failure.error)
        .bind(failure.attempts)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::new)?;

        sqlx::query(UPSERT_POSITION)
            .bind(subscriber)
            .bind(failure.event.0)
            .execute(&mut *tx)
            .await
            .map_err(StorageError::new)?;

        tx.commit().await.map_err(StorageError::new)
    }

    async fn failed(&self, subscriber: &str) -> Result<Vec<(Recorded, DeliveryFailure)>, StorageError> {
        let rows: Vec<FailedRow> = sqlx::query_as(
            "SELECT events.id, events.payload, events.occurred_at, events.correlation, failed_deliveries.error, failed_deliveries.attempts
             FROM failed_deliveries JOIN events ON events.id = failed_deliveries.event_id
             WHERE failed_deliveries.subscriber = ? ORDER BY events.id",
        )
        .bind(subscriber)
        .fetch_all(self.pool())
        .await
        .map_err(StorageError::new)?;

        rows.into_iter()
            .map(|row| {
                let failure =
                    DeliveryFailure { event: EventId(row.event.id), error: row.error, attempts: row.attempts };
                Ok((row.event.try_into()?, failure))
            })
            .collect()
    }

    async fn record_failure(&self, subscriber: &str, failure: &DeliveryFailure) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE failed_deliveries
             SET error = ?, attempts = ?, failed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE subscriber = ? AND event_id = ?",
        )
        .bind(&failure.error)
        .bind(failure.attempts)
        .bind(subscriber)
        .bind(failure.event.0)
        .execute(self.pool())
        .await
        .map_err(StorageError::new)?;
        Ok(())
    }

    async fn resolve(&self, subscriber: &str, event: EventId) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM failed_deliveries WHERE subscriber = ? AND event_id = ?")
            .bind(subscriber)
            .bind(event.0)
            .execute(self.pool())
            .await
            .map_err(StorageError::new)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    payload: Json<Event>,
    occurred_at: String,
    correlation: Option<String>,
}

#[derive(sqlx::FromRow)]
struct FailedRow {
    #[sqlx(flatten)]
    event: EventRow,
    error: String,
    attempts: u32,
}

impl TryFrom<EventRow> for Recorded {
    type Error = StorageError;

    fn try_from(row: EventRow) -> Result<Self, StorageError> {
        Ok(Self {
            id: EventId(row.id),
            occurred_at: row.occurred_at.parse().map_err(StorageError::new)?,
            event: row.payload.0,
            correlation: row.correlation.map(|text| text.parse()).transpose().map_err(StorageError::new)?,
        })
    }
}
