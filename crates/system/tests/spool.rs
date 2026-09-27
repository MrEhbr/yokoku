use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use tempfile::TempDir;
use tokio::sync::Notify;
use yokoku_domain::{SeriesId, StorageError};
use yokoku_events::{Event, EventId, EventLog, EventSpool, Failure, Recorded, SeriesAdded};
use yokoku_system::FileSpool;

#[derive(Default)]
struct MemoryLog {
    events: Mutex<Vec<Event>>,
    failing: AtomicBool,
}

#[async_trait]
impl EventLog for MemoryLog {
    async fn append(&self, events: &[Event]) -> Result<(), StorageError> {
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

fn series_added(title: &str) -> Event {
    SeriesAdded { series: SeriesId::generate(), title: title.into() }.into()
}

#[tokio::test]
async fn replay_appends_every_pushed_event_in_order_and_empties_the_spool() {
    let dir = TempDir::new().unwrap();
    let spool = FileSpool::new(dir.path().join("yokoku.spool"));
    let log = MemoryLog::default();
    let events = [series_added("first"), series_added("second"), series_added("third")];

    spool.push(&events[..2]).await.unwrap();
    spool.push(&events[2..]).await.unwrap();
    let replayed = spool.replay(&log).await.unwrap();

    assert_eq!(replayed, 3);
    assert_eq!(*log.events.lock().unwrap(), events);
    assert_eq!(spool.replay(&log).await.unwrap(), 0);
}

#[tokio::test]
async fn a_refused_replay_keeps_the_spool() {
    let dir = TempDir::new().unwrap();
    let spool = FileSpool::new(dir.path().join("yokoku.spool"));
    let log = MemoryLog { failing: AtomicBool::new(true), ..MemoryLog::default() };
    let event = series_added("first");
    spool.push(std::slice::from_ref(&event)).await.unwrap();

    assert!(spool.replay(&log).await.is_err());
    log.failing.store(false, Ordering::SeqCst);

    assert_eq!(spool.replay(&log).await.unwrap(), 1);
    assert_eq!(*log.events.lock().unwrap(), [event]);
}

#[tokio::test]
async fn replaying_without_a_spool_file_appends_nothing_and_creates_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("yokoku.spool");

    assert_eq!(FileSpool::new(path.clone()).replay(&MemoryLog::default()).await.unwrap(), 0);
    assert!(!path.exists());
}

#[tokio::test]
async fn a_torn_line_is_skipped() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("yokoku.spool");
    let spool = FileSpool::new(path.clone());
    let event = series_added("first");
    spool.push(std::slice::from_ref(&event)).await.unwrap();
    let mut text = fs::read_to_string(&path).unwrap();
    text.push_str("{\"type\":\"SeriesAdd");
    fs::write(&path, text).unwrap();
    let log = MemoryLog::default();

    assert_eq!(spool.replay(&log).await.unwrap(), 1);
    assert_eq!(*log.events.lock().unwrap(), [event]);
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
}

/// Holds each append until `release` is notified; `entered` is notified when one starts.
#[derive(Default)]
struct GatedLog {
    inner: MemoryLog,
    entered: Notify,
    release: Notify,
}

#[async_trait]
impl EventLog for GatedLog {
    async fn append(&self, events: &[Event]) -> Result<(), StorageError> {
        self.entered.notify_one();
        self.release.notified().await;
        self.inner.append(events).await
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
async fn a_push_during_a_replay_waits_for_it_and_is_kept_for_the_next() {
    let dir = TempDir::new().unwrap();
    let spool = FileSpool::new(dir.path().join("yokoku.spool"));
    let log = Arc::new(GatedLog::default());
    let (first, second) = (series_added("first"), series_added("second"));
    spool.push(std::slice::from_ref(&first)).await.unwrap();

    let replay = tokio::spawn({
        let (spool, log) = (spool.clone(), log.clone());
        async move { spool.replay(log.as_ref()).await }
    });
    log.entered.notified().await;
    let push = tokio::spawn({
        let (spool, second) = (spool.clone(), second.clone());
        async move { spool.push(&[second]).await }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!push.is_finished());
    log.release.notify_one();

    assert_eq!(replay.await.unwrap().unwrap(), 1);
    push.await.unwrap().unwrap();
    assert_eq!(spool.replay(&log.inner).await.unwrap(), 1);
    assert_eq!(*log.inner.events.lock().unwrap(), [first, second]);
}
