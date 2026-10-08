//! Torrents, download client sync, seeding cleanup and release search.

mod error;
mod model;
pub mod ports;
mod search;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
pub use error::DownloadError;
pub use model::{Download, DownloadState, TorrentStatus};
pub use search::ReleaseSearch;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};
use yokoku_domain::{
    Clock, DiskSpace, DownloadId, ItemId, Live, StorageError,
    events::{DownloadCompleted, Event, FilesImported, TorrentAdded, TorrentRemoved},
};

use self::ports::{DownloadClient, DownloadRepo, LABEL, Torrent, TorrentSource};
use crate::{
    events::{Handler, HandlerError, Publisher, QueueChanges},
    media::ports::Catalog,
};

/// Torrents Yokoku tracks and their state in the download client.
pub struct Downloads {
    repo: Arc<dyn DownloadRepo>,
    catalog: Arc<dyn Catalog>,
    client: Arc<dyn DownloadClient>,
    clock: Arc<dyn Clock>,
    options: Live<DownloadOptions>,
    events: Publisher,
    changes: QueueChanges,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DownloadOptions {
    /// Removes a torrent with its data once it is imported and the client finished seeding it.
    pub remove_after_seeding: bool,
    /// Torrents added to the client outside Yokoku with any of these labels are taken on.
    #[serde(default)]
    pub pick_up_labels: Vec<String>,
    /// Torrents added to the client outside Yokoku that download at or under this folder are taken on.
    pub pick_up_folder: Option<PathBuf>,
}

impl DownloadOptions {
    /// Labelled `LABEL`, or with a pick up label or folder.
    fn takes_on(&self, torrent: &Torrent) -> bool {
        torrent.labels.iter().any(|label| label == LABEL)
            || self.pick_up_labels.iter().any(|label| torrent.labels.contains(label))
            || self.pick_up_folder.as_ref().is_some_and(|folder| torrent.status.download_dir.starts_with(folder))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub synced: usize,
    /// Downloads that finished since the last sync.
    pub completed: Vec<DownloadId>,
    /// Downloads the client no longer has.
    pub removed: usize,
    /// Imported downloads removed from the client after seeding.
    pub cleaned_up: usize,
    /// Torrents added outside Yokoku and taken on.
    pub picked_up: usize,
}

impl Downloads {
    pub fn new(
        repo: Arc<dyn DownloadRepo>,
        catalog: Arc<dyn Catalog>,
        client: Arc<dyn DownloadClient>,
        clock: Arc<dyn Clock>,
        options: Live<DownloadOptions>,
        events: Publisher,
        changes: QueueChanges,
    ) -> Self {
        Self { repo, catalog, client, clock, options, events, changes }
    }

    /// Newest first.
    pub async fn list(&self) -> Result<Vec<Download>, DownloadError> {
        Ok(self.repo.list().await?)
    }

    /// The space left in the folder new torrents download to.
    pub async fn space(&self) -> Result<DiskSpace, DownloadError> {
        Ok(self.client.space().await?)
    }

    /// Adds a torrent for `item`, or for detection to work out when `None`; for a series,
    /// `season` places its files whose names give none.
    #[instrument(skip_all, fields(item = ?item, season = ?season))]
    pub async fn add(
        &self,
        torrent: &TorrentSource,
        item: Option<ItemId>,
        season: Option<u16>,
    ) -> Result<Download, DownloadError> {
        if let Some(ItemId::Movie(id)) = item {
            if self.catalog.movie(id).await?.is_some_and(|movie| movie.file.is_some())
                || self
                    .repo
                    .list()
                    .await?
                    .iter()
                    .any(|download| download.item == item && download.status.state != DownloadState::Removed)
            {
                return Err(DownloadError::MovieAlreadyHasDownload);
            }
        }
        let added = self.client.add(torrent).await?;
        if self.repo.find_by_hash(&added.hash).await?.is_some() {
            return Err(DownloadError::AlreadyAdded(added.name));
        }

        let torrent = self.client.torrents(std::slice::from_ref(&added.hash)).await?.pop();
        let (mut download, events) = self.take_on(added.hash, added.name, item, season, torrent);
        self.repo.save(&mut download).await?;
        self.changes.notify();
        info!(download = %download.id, name = %download.name, "torrent added");
        self.events.publish_all(events).await;
        Ok(download)
    }

    /// Records each download's progress and state; a download that finished emits
    /// `DownloadCompleted` once, however often and however many syncs run. A download another
    /// sync saved in the meantime is left to that sync. With `remove_after_seeding`, an imported
    /// download whose seeding finished is removed from the client, emitting `TorrentRemoved`. Unknown
    /// torrents labelled `LABEL`, which Yokoku added but never saved, or matching `pick_up_labels` or
    /// `pick_up_folder`, become unlinked downloads, emitting `TorrentAdded`.
    #[instrument(skip_all)]
    pub async fn sync(&self) -> Result<SyncReport, DownloadError> {
        let options = self.options.current();
        let known = self.repo.list().await?;
        let active: Vec<Download> =
            known.iter().filter(|download| download.status.state != DownloadState::Removed).cloned().collect();
        let mut torrents: HashMap<String, Torrent> =
            self.client.all_torrents().await?.into_iter().map(|torrent| (torrent.hash.clone(), torrent)).collect();

        let mut report = SyncReport::default();
        for mut download in active {
            let torrent = torrents.remove(&download.hash);
            let removed = torrent.is_none();
            let seeded = torrent.as_ref().is_some_and(|torrent| torrent.seeding_done);
            let completed = self.apply(&mut download, torrent);
            let mut events: Vec<Event> = completed.iter().cloned().collect();
            let clean_up = seeded && options.remove_after_seeding && download.imported_at.is_some();
            if clean_up {
                self.client.remove(&download.hash, true).await?;
                download.mark_removed();
                events.push(
                    TorrentRemoved { download: download.id, name: download.name.clone(), item: download.item }.into(),
                );
            }
            match self.repo.save(&mut download).await {
                Err(StorageError::Conflict) => {
                    debug!(download = %download.id, "another sync saved the download first");
                    continue;
                },
                result => result?,
            }
            self.changes.notify();
            self.events.publish_all(events).await;
            let (id, name) = (&download.id, &download.name);
            if completed.is_some() {
                info!(download = %id, name, "download completed");
            }
            if removed {
                info!(download = %id, name, "torrent is gone from the client");
            }
            if clean_up {
                info!(download = %id, name, "torrent removed after seeding");
            }
            report.synced += 1;
            report.removed += usize::from(removed);
            report.cleaned_up += usize::from(clean_up);
            if completed.is_some() {
                report.completed.push(download.id);
            }
        }

        let known: HashSet<&str> = known.iter().map(|download| download.hash.as_str()).collect();
        let mut new: Vec<Torrent> = torrents
            .into_values()
            .filter(|torrent| !known.contains(torrent.hash.as_str()) && options.takes_on(torrent))
            .collect();
        new.sort_by(|a, b| a.hash.cmp(&b.hash));
        for torrent in new {
            let (mut download, events) =
                self.take_on(torrent.hash.clone(), torrent.name.clone(), None, None, Some(torrent));
            match self.repo.save(&mut download).await {
                Err(StorageError::Conflict) => {
                    debug!(hash = %download.hash, "another sync took the torrent on first");
                    continue;
                },
                result => result?,
            }
            self.changes.notify();
            self.events.publish_all(events).await;
            info!(download = %download.id, name = %download.name, "torrent taken on");
            report.picked_up += 1;
            if download.completed_at.is_some() {
                report.completed.push(download.id);
            }
        }
        debug!(synced = report.synced, picked_up = report.picked_up, "downloads synced");
        Ok(report)
    }

    /// A new download and its `TorrentAdded`, with `DownloadCompleted` when the torrent is complete.
    fn take_on(
        &self,
        hash: String,
        name: String,
        item: Option<ItemId>,
        season: Option<u16>,
        torrent: Option<Torrent>,
    ) -> (Download, Vec<Event>) {
        let mut download = Download {
            id: DownloadId::generate(),
            hash,
            name,
            item,
            season,
            status: TorrentStatus::unknown(),
            added_at: self.clock.now().timestamp(),
            completed_at: None,
            imported_at: None,
            revision: 0,
        };
        let mut events = vec![TorrentAdded { download: download.id, name: download.name.clone(), item }.into()];
        events.extend(self.apply(&mut download, torrent));
        (download, events)
    }

    /// Syncs like `sync`, but only while a download is queued, checking or downloading; `None` when
    /// none is.
    pub async fn sync_active(&self) -> Result<Option<SyncReport>, DownloadError> {
        let active = self.repo.list().await?.iter().any(|download| {
            matches!(
                download.status.state,
                DownloadState::Queued | DownloadState::Checking | DownloadState::Downloading
            )
        });
        if !active {
            return Ok(None);
        }
        self.sync().await.map(Some)
    }

    /// Records that files from the download reached the library; the first import counts.
    pub async fn mark_imported(&self, id: DownloadId) -> Result<(), DownloadError> {
        let Some(mut download) = self.repo.get(id).await? else { return Ok(()) };
        if download.imported_at.is_none() {
            download.imported_at = Some(self.clock.now().timestamp());
            self.repo.save(&mut download).await?;
            self.changes.notify();
        }
        Ok(())
    }

    /// Returns `DownloadCompleted` when the download has just finished.
    fn apply(&self, download: &mut Download, torrent: Option<Torrent>) -> Option<Event> {
        let Some(torrent) = torrent else {
            download.mark_removed();
            return None;
        };
        download.name = torrent.name;
        download.status = torrent.status;
        if !torrent.complete || download.completed_at.is_some() {
            return None;
        }
        download.completed_at = Some(self.clock.now().timestamp());
        Some(
            DownloadCompleted {
                download: download.id,
                name: download.name.clone(),
                content_path: download.content_path(),
                item: download.item,
                season: download.season,
            }
            .into(),
        )
    }
}

#[async_trait]
impl Handler<FilesImported> for Downloads {
    async fn handle(&self, event: &FilesImported) -> Result<(), HandlerError> {
        if let Some(download) = event.download {
            self.mark_imported(download).await?;
        }
        Ok(())
    }
}
