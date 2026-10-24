//! Notifications about the library, sent through a `Notifier`.

pub mod ports;

use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::Write,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use async_trait::async_trait;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use yokoku_domain::{
    Clock, FileTarget, ItemId, ItemName, Live,
    events::{DownloadCompleted, Event, FilesImported, ImportFailed, ImportNeedsReview, TorrentAdded},
};

use self::ports::{Notification, Notifier, NotifyError, Priority};
use crate::{
    events::{HandlerError, Subscriber},
    jobs::{Job, JobError},
    library::ports::{MovieRepo, SeriesRepo},
};

/// A notification waits this long without another of its kind about the same item before it is
/// sent.
const QUIET: SignedDuration = SignedDuration::from_secs(30);
/// Lines a notification lists; the rest are counted.
const LISTED: usize = 10;
/// Sends tried before a notification is dropped.
const ATTEMPTS: u32 = 5;

/// Which events notify.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct NotificationSettings {
    /// Files placed in the library.
    pub imported: bool,
    pub needs_review: bool,
    pub failed: bool,
    /// Torrents added to the download client.
    pub grabbed: bool,
    /// Torrents that finished downloading.
    pub downloaded: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self { imported: true, needs_review: true, failed: true, grabbed: true, downloaded: true }
    }
}

/// Notifies about imports, imports that need review or failed, and torrents grabbed or
/// downloaded. Those of one kind about one item go out as one notification once `QUIET` passes
/// without another; they wait in memory, so a stop loses the ones not sent yet.
pub struct Notifications {
    notifier: Arc<dyn Notifier>,
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    settings: Live<NotificationSettings>,
    clock: Arc<dyn Clock>,
    /// In the order they started.
    pending: Mutex<Vec<Batch>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Imported,
    NeedsReview,
    Failed,
    Grabbed,
    Downloaded,
}

impl Kind {
    /// The title of a notification about no one item.
    fn title(self) -> &'static str {
        match self {
            Self::Imported => "Imported",
            Self::NeedsReview => "Import needs review",
            Self::Failed => "Import failed",
            Self::Grabbed => "Grabbed",
            Self::Downloaded => "Downloaded",
        }
    }

    /// The message about one `line`.
    fn one(self, line: &str) -> String {
        match self {
            Self::Imported => format!("Imported {line}"),
            Self::NeedsReview | Self::Failed => line.to_owned(),
            Self::Grabbed => format!("Grabbed {line}"),
            Self::Downloaded => format!("Downloaded {line}"),
        }
    }

    /// The first line of a message about `count` lines.
    fn many(self, count: usize) -> String {
        match self {
            Self::Imported => format!("Imported {count} files"),
            Self::NeedsReview => format!("{count} imports need review"),
            Self::Failed => format!("{count} imports failed"),
            Self::Grabbed => format!("Grabbed {count} torrents"),
            Self::Downloaded => format!("Downloaded {count} torrents"),
        }
    }

    fn priority(self) -> Priority {
        match self {
            Self::NeedsReview | Self::Failed => Priority::High,
            Self::Imported | Self::Grabbed | Self::Downloaded => Priority::Normal,
        }
    }
}

/// One notification waiting for more lines.
struct Batch {
    kind: Kind,
    item: Option<ItemId>,
    title: String,
    lines: Vec<String>,
    last: Timestamp,
    failed_attempts: u32,
}

impl Batch {
    fn is_for(&self, kind: Kind, item: Option<ItemId>) -> bool {
        self.kind == kind && self.item == item
    }

    fn notification(&self) -> Notification {
        let kind = self.kind;
        let message = match self.lines.as_slice() {
            [line] => kind.one(line),
            lines => {
                let mut message = kind.many(lines.len());
                for line in lines.iter().take(LISTED) {
                    message.push('\n');
                    message.push_str(line);
                }
                if lines.len() > LISTED {
                    _ = write!(message, "\n…and {} more", lines.len() - LISTED);
                }
                message
            },
        };
        Notification { title: self.title.clone(), message, priority: kind.priority() }
    }
}

impl Notifications {
    pub fn new(
        notifier: Arc<dyn Notifier>,
        series: Arc<dyn SeriesRepo>,
        movies: Arc<dyn MovieRepo>,
        settings: Live<NotificationSettings>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { notifier, series, movies, settings, clock, pending: Mutex::new(Vec::new()) }
    }

    fn pending(&self) -> MutexGuard<'_, Vec<Batch>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The item's name, or `None` once it is removed.
    async fn name(&self, item: ItemId) -> Result<Option<String>, HandlerError> {
        Ok(match item {
            ItemId::Series(id) => {
                self.series.get(id).await?.map(|series| ItemName::new(&series.title, series.year).to_string())
            },
            ItemId::Movie(id) => {
                self.movies.get(id).await?.map(|movie| ItemName::new(&movie.title, movie.year).to_string())
            },
        })
    }
}

#[async_trait]
impl Subscriber for Notifications {
    fn name(&self) -> &'static str {
        "notifications"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        let settings = self.settings.current();
        let lines: Vec<(Kind, Option<ItemId>, String)> = match event {
            Event::FilesImported(FilesImported { files, .. }) if settings.imported => files
                .iter()
                .map(|file| {
                    let line = match &file.target {
                        FileTarget::Episodes { span, .. } => span.to_string(),
                        FileTarget::Movie(_) => file.path.file_name().unwrap_or_default().to_string_lossy().into(),
                    };
                    (Kind::Imported, Some(file.target.item()), line)
                })
                .collect(),
            Event::ImportNeedsReview(ImportNeedsReview { source, .. }) if settings.needs_review => {
                vec![(Kind::NeedsReview, None, source.file_name().unwrap_or_default().to_string_lossy().into())]
            },
            Event::ImportFailed(ImportFailed { source, reason, .. }) if settings.failed => {
                let file = source.file_name().unwrap_or_default().to_string_lossy();
                vec![(Kind::Failed, None, format!("{file}: {reason}"))]
            },
            Event::TorrentAdded(TorrentAdded { name, item, .. }) if settings.grabbed => {
                vec![(Kind::Grabbed, *item, name.clone())]
            },
            Event::DownloadCompleted(DownloadCompleted { name, item, .. }) if settings.downloaded => {
                vec![(Kind::Downloaded, *item, name.clone())]
            },
            _ => return Ok(()),
        };
        let mut names = HashMap::new();
        for item in lines.iter().filter_map(|(_, item, _)| *item) {
            if let Entry::Vacant(name) = names.entry(item) {
                name.insert(self.name(item).await?);
            }
        }
        let now = self.clock.now().timestamp();
        let mut pending = self.pending();
        for (kind, item, line) in lines {
            let index = match pending.iter().position(|batch| batch.is_for(kind, item)) {
                Some(index) => index,
                None => {
                    let title = item.and_then(|item| names[&item].clone()).unwrap_or_else(|| kind.title().to_owned());
                    pending.push(Batch { kind, item, title, lines: Vec::new(), last: now, failed_attempts: 0 });
                    pending.len() - 1
                },
            };
            pending[index].lines.push(line);
            pending[index].last = now;
        }
        Ok(())
    }
}

/// Sends the notifications `QUIET` has passed for, oldest first; one that fails waits for the
/// next run, up to `ATTEMPTS` sends.
#[async_trait]
impl Job for Notifications {
    fn name(&self) -> &'static str {
        "send-notifications"
    }

    async fn run(&self) -> Result<(), JobError> {
        let quiet_since = self.clock.now().timestamp() - QUIET;
        let due: Vec<Batch> = {
            let mut pending = self.pending();
            let (due, waiting) = pending.drain(..).partition(|batch| batch.last <= quiet_since);
            *pending = waiting;
            due
        };
        let (mut retries, mut failure) = (Vec::new(), None);
        for mut batch in due {
            match self.notifier.send(&batch.notification()).await {
                Ok(()) => info!(title = batch.title, lines = batch.lines.len(), "notification sent"),
                Err(NotifyError::NotConfigured) => {},
                Err(error) if batch.failed_attempts + 1 >= ATTEMPTS => {
                    warn!(%error, title = batch.title, "notification dropped after repeated failures");
                },
                Err(error) => {
                    batch.failed_attempts += 1;
                    retries.push(batch);
                    failure = Some(error);
                },
            }
        }
        let mut pending = self.pending();
        for batch in &mut retries {
            if let Some(index) = pending.iter().position(|newer| newer.is_for(batch.kind, batch.item)) {
                let newer = pending.remove(index);
                batch.lines.extend(newer.lines);
                batch.last = newer.last;
            }
        }
        pending.splice(0..0, retries);
        failure.map_or(Ok(()), |error| Err(error.into()))
    }
}
