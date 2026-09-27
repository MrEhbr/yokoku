use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use yokoku_domain::{MovieId, SeriesId, StorageError};
use yokoku_events::{
    Correlated, CorrelationId, DeliveryFailure, Event, EventId, EventLog, EventSpool, MovieAdded, Publisher, Recorded,
    SeriesAdded, correlation::correlate,
};

/// Keeps appended events; refuses them while `failing`.
#[derive(Default)]
struct MemoryLog {
    events: Mutex<Vec<Correlated>>,
    failing: AtomicBool,
}

impl MemoryLog {
    fn failing() -> Self {
        Self { failing: AtomicBool::new(true), ..Self::default() }
    }

    fn recover(&self) {
        self.failing.store(false, Ordering::SeqCst);
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().iter().map(|stored| stored.event.clone()).collect()
    }

    fn correlations(&self) -> Vec<CorrelationId> {
        self.events.lock().unwrap().iter().map(|stored| stored.correlation).collect()
    }
}

#[async_trait]
impl EventLog for MemoryLog {
    async fn append(&self, events: &[Correlated]) -> Result<(), StorageError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(StorageError::new(std::io::Error::other("database is locked")));
        }
        self.events.lock().unwrap().extend_from_slice(events);
        Ok(())
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

    async fn give_up(&self, _: &str, _: &DeliveryFailure) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn failed(&self, _: &str) -> Result<Vec<(Recorded, DeliveryFailure)>, StorageError> {
        unreachable!()
    }

    async fn record_failure(&self, _: &str, _: &DeliveryFailure) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn resolve(&self, _: &str, _: EventId) -> Result<(), StorageError> {
        unreachable!()
    }
}

/// Keeps spooled events in memory; refuses them while `failing`.
#[derive(Default)]
struct MemorySpool {
    events: Mutex<Vec<Correlated>>,
    failing: bool,
}

impl MemorySpool {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().iter().map(|stored| stored.event.clone()).collect()
    }
}

#[async_trait]
impl EventSpool for MemorySpool {
    async fn push(&self, events: &[Correlated]) -> Result<(), StorageError> {
        if self.failing {
            return Err(StorageError::new(std::io::Error::other("disk full")));
        }
        self.events.lock().unwrap().extend_from_slice(events);
        Ok(())
    }

    async fn replay(&self, log: &dyn EventLog) -> Result<usize, StorageError> {
        let spooled = self.events.lock().unwrap().clone();
        if spooled.is_empty() {
            return Ok(0);
        }
        log.append(&spooled).await?;
        self.events.lock().unwrap().clear();
        Ok(spooled.len())
    }
}

fn series_added(title: &str) -> Event {
    SeriesAdded { series: SeriesId::generate(), title: title.into() }.into()
}

fn publisher(log: &Arc<MemoryLog>, spool: &Arc<MemorySpool>) -> Publisher {
    Publisher::new(log.clone(), spool.clone())
}

#[tokio::test]
async fn published_events_are_appended_in_order() {
    let (log, spool) = (Arc::new(MemoryLog::default()), Arc::new(MemorySpool::default()));
    let added = series_added("Frieren");
    let movie: Event = MovieAdded { movie: MovieId::generate(), title: "Dune".into() }.into();

    publisher(&log, &spool).publish(added.clone()).await;
    publisher(&log, &spool).publish_all(vec![movie.clone()]).await;

    assert_eq!(log.events(), [added, movie]);
    assert!(spool.events().is_empty());
}

#[tokio::test]
async fn publishing_nothing_appends_nothing() {
    let (log, spool) = (Arc::new(MemoryLog::default()), Arc::new(MemorySpool::default()));

    publisher(&log, &spool).publish_all(Vec::new()).await;

    assert!(log.events().is_empty());
}

#[tokio::test]
async fn events_the_log_refuses_are_spooled_and_appended_first_once_it_recovers() {
    let (log, spool) = (Arc::new(MemoryLog::failing()), Arc::new(MemorySpool::default()));
    let (first, second) = (series_added("first"), series_added("second"));

    publisher(&log, &spool).publish(first.clone()).await;
    assert_eq!(spool.events(), std::slice::from_ref(&first));
    log.recover();
    publisher(&log, &spool).publish(second.clone()).await;

    assert_eq!(log.events(), [first, second]);
    assert!(spool.events().is_empty());
}

#[tokio::test]
async fn new_events_wait_behind_spooled_ones_while_the_log_is_down() {
    let (log, spool) = (Arc::new(MemoryLog::failing()), Arc::new(MemorySpool::default()));
    let (first, second) = (series_added("first"), series_added("second"));

    publisher(&log, &spool).publish(first.clone()).await;
    publisher(&log, &spool).publish(second.clone()).await;

    assert!(log.events().is_empty());
    assert_eq!(spool.events(), [first, second]);
}

#[tokio::test]
async fn replaying_appends_spooled_events_without_a_new_publish() {
    let (log, spool) = (Arc::new(MemoryLog::failing()), Arc::new(MemorySpool::default()));
    let first = series_added("first");
    publisher(&log, &spool).publish(first.clone()).await;

    log.recover();
    let caught_up = publisher(&log, &spool).replay().await;

    assert!(caught_up);
    assert_eq!(log.events(), [first]);
    assert!(spool.events().is_empty());
}

#[tokio::test]
async fn events_are_lost_without_failing_when_neither_log_nor_spool_takes_them() {
    let log = Arc::new(MemoryLog::failing());
    let spool = Arc::new(MemorySpool { failing: true, ..MemorySpool::default() });

    publisher(&log, &spool).publish(series_added("Frieren")).await;

    assert!(log.events().is_empty() && spool.events().is_empty());
}

#[tokio::test]
async fn events_carry_the_correlation_id_they_are_published_under() {
    let (log, spool) = (Arc::new(MemoryLog::default()), Arc::new(MemorySpool::default()));
    let correlation = CorrelationId::generate();

    correlate(correlation, publisher(&log, &spool).publish_all(vec![series_added("a"), series_added("b")])).await;

    assert_eq!(log.correlations(), [correlation; 2]);
}

#[tokio::test]
async fn events_published_outside_a_correlation_share_a_new_one_per_publish() {
    let (log, spool) = (Arc::new(MemoryLog::default()), Arc::new(MemorySpool::default()));

    publisher(&log, &spool).publish_all(vec![series_added("a"), series_added("b")]).await;
    publisher(&log, &spool).publish(series_added("c")).await;

    let correlations = log.correlations();
    assert_eq!(correlations[0], correlations[1]);
    assert_ne!(correlations[1], correlations[2]);
}
