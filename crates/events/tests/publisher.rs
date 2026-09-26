use std::sync::Arc;

use async_trait::async_trait;
use yokoku_db::Database;
use yokoku_domain::{MovieId, SeriesId, StorageError};
use yokoku_events::{Event, EventId, EventLog, Failure, MovieAdded, Publisher, Recorded, SeriesAdded};

fn series_added() -> SeriesAdded {
    SeriesAdded { series: SeriesId::generate(), title: "Frieren".into() }
}

async fn logged(db: &Database) -> Vec<Event> {
    db.event_log().read_after(None, 10).await.unwrap().into_iter().map(|recorded| recorded.event).collect()
}

#[tokio::test]
async fn published_events_are_appended_in_order() {
    let db = Database::open_in_memory().await.unwrap();
    let publisher = Publisher::new(Arc::new(db.event_log()));
    let added = series_added();
    let movie = MovieAdded { movie: MovieId::generate(), title: "Dune".into() };

    publisher.publish(added.clone()).await;
    publisher.publish_all(vec![movie.clone().into()]).await;

    assert_eq!(logged(&db).await, [added.into(), movie.into()]);
}

#[tokio::test]
async fn publishing_nothing_appends_nothing() {
    let db = Database::open_in_memory().await.unwrap();

    Publisher::new(Arc::new(db.event_log())).publish_all(Vec::new()).await;

    assert!(logged(&db).await.is_empty());
}

struct Unavailable;

#[async_trait]
impl EventLog for Unavailable {
    async fn append(&self, _: &[Event]) -> Result<(), StorageError> {
        Err(StorageError::new(std::io::Error::other("disk full")))
    }

    async fn last_delivered(&self, _: &str) -> Result<Option<EventId>, StorageError> {
        unreachable!()
    }

    async fn read_after(&self, _: Option<EventId>, _: u32) -> Result<Vec<Recorded>, StorageError> {
        unreachable!()
    }

    async fn read_before(&self, _: Option<EventId>, _: u32) -> Result<Vec<Recorded>, StorageError> {
        unreachable!()
    }

    async fn mark_delivered(&self, _: &str, _: EventId) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn give_up(&self, _: &str, _: &Failure) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn failed(&self, _: &str) -> Result<Vec<(Recorded, Failure)>, StorageError> {
        unreachable!()
    }

    async fn record_failure(&self, _: &str, _: &Failure) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn resolve(&self, _: &str, _: EventId) -> Result<(), StorageError> {
        unreachable!()
    }
}

#[tokio::test]
async fn a_log_that_cannot_append_loses_the_events_without_failing() {
    Publisher::new(Arc::new(Unavailable)).publish(series_added()).await;
}
