use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::PathBuf,
};

use async_trait::async_trait;
use tokio::task;
use tracing::error;
use yokoku_domain::StorageError;
use yokoku_events::{Event, EventLog, EventSpool};

/// Spooled events as JSON lines in one file; every access holds an exclusive `flock` on it, in
/// this process or another.
#[derive(Debug, Clone)]
pub struct FileSpool {
    path: PathBuf,
}

impl FileSpool {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl EventSpool for FileSpool {
    async fn push(&self, events: &[Event]) -> Result<(), StorageError> {
        let mut lines = String::new();
        for event in events {
            lines.push_str(&serde_json::to_string(event).map_err(StorageError::new)?);
            lines.push('\n');
        }
        let path = self.path.clone();
        blocking(move || {
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            file.lock()?;
            file.write_all(lines.as_bytes())?;
            file.sync_all()
        })
        .await
    }

    async fn replay(&self, log: &dyn EventLog) -> Result<usize, StorageError> {
        let path = self.path.clone();
        let Some((file, events)) = blocking(move || {
            let mut file = match OpenOptions::new().read(true).write(true).open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            };
            file.lock()?;
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            Ok(Some((file, decode(&text))))
        })
        .await?
        else {
            return Ok(0);
        };
        if !events.is_empty() {
            log.append(&events).await?;
        }
        blocking(move || empty(&file)).await?;
        Ok(events.len())
    }
}

/// Lines that are not an event, such as one torn by a stop mid-write, are logged and skipped.
fn decode(text: &str) -> Vec<Event> {
    text.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            serde_json::from_str(line)
                .inspect_err(|error| error!(%error, line, "skipping a spooled line that is not an event"))
                .ok()
        })
        .collect()
}

fn empty(file: &File) -> io::Result<()> {
    file.set_len(0)?;
    file.sync_all()
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> io::Result<T> + Send + 'static) -> Result<T, StorageError> {
    task::spawn_blocking(work).await.map_err(StorageError::new)?.map_err(StorageError::new)
}
