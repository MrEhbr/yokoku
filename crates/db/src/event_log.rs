use async_trait::async_trait;
use sqlx::{SqliteConnection, SqlitePool};
use yokoku_events::{Event, EventId, EventLog, EventLogError, Failure, Recorded};

use crate::DbError;

const UPSERT_POSITION: &str = "
    INSERT INTO subscriber_positions (subscriber, last_event_id) VALUES (?, ?)
    ON CONFLICT (subscriber) DO UPDATE SET last_event_id = excluded.last_event_id";

pub(crate) async fn append(conn: &mut SqliteConnection, events: &[Event]) -> Result<(), DbError> {
    for event in events {
        let payload = serde_json::to_string(event)?;
        sqlx::query("INSERT INTO events (payload) VALUES (?)").bind(payload).execute(&mut *conn).await?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SqliteEventLog {
    pool: SqlitePool,
}

impl SqliteEventLog {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn failed_deliveries(&self, subscriber: &str) -> Result<Vec<Failure>, DbError> {
        let rows: Vec<(i64, String, u32)> = sqlx::query_as(
            "SELECT event_id, error, attempts FROM failed_deliveries WHERE subscriber = ? ORDER BY event_id",
        )
        .bind(subscriber)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(event, error, attempts)| Failure { event: EventId(event), error, attempts })
            .collect())
    }
}

#[async_trait]
impl EventLog for SqliteEventLog {
    async fn last_delivered(&self, subscriber: &str) -> Result<Option<EventId>, EventLogError> {
        let position: Option<i64> =
            sqlx::query_scalar("SELECT last_event_id FROM subscriber_positions WHERE subscriber = ?")
                .bind(subscriber)
                .fetch_optional(&self.pool)
                .await
                .map_err(EventLogError::new)?;

        Ok(position.map(EventId))
    }

    async fn read_after(&self, after: Option<EventId>, limit: u32) -> Result<Vec<Recorded>, EventLogError> {
        let rows: Vec<(i64, String, String)> =
            sqlx::query_as("SELECT id, payload, occurred_at FROM events WHERE id > ? ORDER BY id LIMIT ?")
                .bind(after.map_or(0, |id| id.0))
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(EventLogError::new)?;

        rows.into_iter()
            .map(|(id, payload, occurred_at)| {
                Ok(Recorded {
                    id: EventId(id),
                    occurred_at: occurred_at.parse().map_err(EventLogError::new)?,
                    event: serde_json::from_str(&payload).map_err(EventLogError::new)?,
                })
            })
            .collect()
    }

    async fn mark_delivered(&self, subscriber: &str, event: EventId) -> Result<(), EventLogError> {
        sqlx::query(UPSERT_POSITION)
            .bind(subscriber)
            .bind(event.0)
            .execute(&self.pool)
            .await
            .map_err(EventLogError::new)?;

        Ok(())
    }

    async fn give_up(&self, subscriber: &str, failure: &Failure) -> Result<(), EventLogError> {
        let mut tx = self.pool.begin().await.map_err(EventLogError::new)?;

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
        .map_err(EventLogError::new)?;

        sqlx::query(UPSERT_POSITION)
            .bind(subscriber)
            .bind(failure.event.0)
            .execute(&mut *tx)
            .await
            .map_err(EventLogError::new)?;

        tx.commit().await.map_err(EventLogError::new)
    }
}
