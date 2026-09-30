use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use yokoku_db::Database;
use yokoku_domain::StorageError;
use yokoku_events::{Correlated, CorrelationId, DeliveryFailure, Event, EventId, EventLog, Publisher, Recorded};
use yokoku_system::FileSpool;

/// Publishes to `db`'s event log, spooling to `dir/yokoku.spool`.
pub fn publisher(db: &Database, dir: &Path) -> Publisher {
    Publisher::new(Arc::new(db.event_log()), Arc::new(FileSpool::new(dir.join("yokoku.spool"))))
}

/// Keeps appended events; refuses them while failing. Reading is not supported.
#[derive(Default)]
pub struct MemoryLog {
    events: Mutex<Vec<Correlated>>,
    failing: AtomicBool,
}

impl MemoryLog {
    pub fn failing() -> Self {
        Self { failing: AtomicBool::new(true), ..Self::default() }
    }

    pub fn recover(&self) {
        self.failing.store(false, Ordering::SeqCst);
    }

    pub fn appended(&self) -> Vec<Correlated> {
        self.events.lock().unwrap().clone()
    }

    pub fn events(&self) -> Vec<Event> {
        self.appended().into_iter().map(|stored| stored.event).collect()
    }

    pub fn correlations(&self) -> Vec<CorrelationId> {
        self.appended().into_iter().map(|stored| stored.correlation).collect()
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
