use yokoku_core::events::{EventLog, Publisher};
use yokoku_infra::db::Database;

pub fn publisher(db: &Database) -> Publisher {
    Publisher::new(EventLog::new(db.clone()))
}

/// Makes `db`'s event log refuse every append until `accept_events`.
pub async fn refuse_events(db: &Database) {
    execute(
        db,
        "CREATE TRIGGER refuse_events BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'events refused'); END",
    )
    .await;
}

pub async fn accept_events(db: &Database) {
    execute(db, "DROP TRIGGER refuse_events").await;
}

async fn execute(db: &Database, sql: &'static str) {
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query(sql).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}
