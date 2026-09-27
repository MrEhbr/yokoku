use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use yokoku_domain::{Clock, DownloadId, ItemId, StorageError};
use yokoku_events::{
    DownloadCompleted, Event, FilesImported, Handler, HandlerError, Publisher, TorrentAdded, TorrentRemoved,
};

use crate::{
    Download, DownloadError, DownloadState, DownloadStatus,
    ports::{DownloadClient, DownloadRepo, Torrent, TorrentSource},
};

/// Torrents added through Yokoku and their state in the download client (FR-3).
pub struct Downloads {
    repo: Arc<dyn DownloadRepo>,
    client: Arc<dyn DownloadClient>,
    clock: Arc<dyn Clock>,
    options: DownloadOptions,
    events: Publisher,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadOptions {
    /// Removes a torrent with its data once it is imported and the client finished seeding it (FR-3.7).
    pub remove_after_seeding: bool,
    /// Torrents added to the client outside Yokoku that sync takes on (FR-3.3).
    pub pick_up: Option<PickUp>,
}

/// A torrent qualifies with any of `labels`, or a download folder at or under `folder`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickUp {
    pub labels: Vec<String>,
    pub folder: Option<PathBuf>,
}

impl PickUp {
    fn matches(&self, torrent: &Torrent) -> bool {
        self.labels.iter().any(|label| torrent.labels.contains(label))
            || self.folder.as_ref().is_some_and(|folder| torrent.status.download_dir.starts_with(folder))
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
        client: Arc<dyn DownloadClient>,
        clock: Arc<dyn Clock>,
        options: DownloadOptions,
        events: Publisher,
    ) -> Self {
        Self { repo, client, clock, options, events }
    }

    /// The client's name and version (FR-3.1).
    pub async fn test_connection(&self) -> Result<String, DownloadError> {
        Ok(self.client.version().await?)
    }

    /// Newest first.
    pub async fn list(&self) -> Result<Vec<Download>, DownloadError> {
        Ok(self.repo.list().await?)
    }

    /// Adds a torrent for `item`, or for detection to work out when `None` (FR-3.2).
    pub async fn add(&self, torrent: &TorrentSource, item: Option<ItemId>) -> Result<Download, DownloadError> {
        let added = self.client.add(torrent).await?;
        if self.repo.find_by_hash(&added.hash).await?.is_some() {
            return Err(DownloadError::AlreadyAdded(added.name));
        }

        let torrent = self.client.torrents(std::slice::from_ref(&added.hash)).await?.pop();
        let (mut download, events) = self.take_on(added.hash, added.name, item, torrent);
        self.repo.save(&mut download).await?;
        self.events.publish_all(events).await;
        Ok(download)
    }

    /// Records each download's progress and state; a download that finished emits
    /// `DownloadCompleted` once, however often and however many syncs run. A download another
    /// sync saved in the meantime is left to that sync. With `remove_after_seeding`, an imported
    /// download whose seeding finished is removed from the client, emitting `TorrentRemoved`. With
    /// `pick_up`, unknown torrents that qualify become unlinked downloads, emitting `TorrentAdded`.
    pub async fn sync(&self) -> Result<SyncReport, DownloadError> {
        let known = self.repo.list().await?;
        let active: Vec<Download> =
            known.iter().filter(|download| download.status.state != DownloadState::Removed).cloned().collect();
        let torrents = match &self.options.pick_up {
            Some(_) => self.client.all_torrents().await?,
            None => {
                let hashes: Vec<String> = active.iter().map(|download| download.hash.clone()).collect();
                self.client.torrents(&hashes).await?
            },
        };
        let mut torrents: HashMap<String, Torrent> =
            torrents.into_iter().map(|torrent| (torrent.hash.clone(), torrent)).collect();

        let mut report = SyncReport::default();
        for mut download in active {
            let torrent = torrents.remove(&download.hash);
            let removed = torrent.is_none();
            let seeded = torrent.as_ref().is_some_and(|torrent| torrent.seeding_done);
            let completed = self.apply(&mut download, torrent);
            let mut events: Vec<Event> = completed.iter().cloned().collect();
            let clean_up = seeded && self.options.remove_after_seeding && download.imported_at.is_some();
            if clean_up {
                self.client.remove(&download.hash, true).await?;
                download.mark_removed();
                events.push(
                    TorrentRemoved { download: download.id, name: download.name.clone(), item: download.item }.into(),
                );
            }
            match self.repo.save(&mut download).await {
                Err(StorageError::Conflict) => continue,
                result => result?,
            }
            self.events.publish_all(events).await;
            report.synced += 1;
            report.removed += usize::from(removed);
            report.cleaned_up += usize::from(clean_up);
            if completed.is_some() {
                report.completed.push(download.id);
            }
        }

        if let Some(pick_up) = &self.options.pick_up {
            let known: HashSet<&str> = known.iter().map(|download| download.hash.as_str()).collect();
            let mut new: Vec<Torrent> = torrents
                .into_values()
                .filter(|torrent| !known.contains(torrent.hash.as_str()) && pick_up.matches(torrent))
                .collect();
            new.sort_by(|a, b| a.hash.cmp(&b.hash));
            for torrent in new {
                let (mut download, events) =
                    self.take_on(torrent.hash.clone(), torrent.name.clone(), None, Some(torrent));
                match self.repo.save(&mut download).await {
                    Err(StorageError::Conflict) => continue,
                    result => result?,
                }
                self.events.publish_all(events).await;
                report.picked_up += 1;
                if download.completed_at.is_some() {
                    report.completed.push(download.id);
                }
            }
        }
        Ok(report)
    }

    /// A new download and its `TorrentAdded`, with `DownloadCompleted` when the torrent is complete.
    fn take_on(
        &self,
        hash: String,
        name: String,
        item: Option<ItemId>,
        torrent: Option<Torrent>,
    ) -> (Download, Vec<Event>) {
        let mut download = Download {
            id: DownloadId::generate(),
            hash,
            name,
            item,
            status: DownloadStatus::unknown(),
            added_at: self.clock.now().timestamp(),
            completed_at: None,
            imported_at: None,
            revision: 0,
        };
        let mut events = vec![TorrentAdded { download: download.id, name: download.name.clone(), item }.into()];
        events.extend(self.apply(&mut download, torrent));
        (download, events)
    }

    /// Records that files from the download reached the library; the first import counts.
    pub async fn mark_imported(&self, id: DownloadId) -> Result<(), DownloadError> {
        let Some(mut download) = self.repo.get(id).await? else { return Ok(()) };
        if download.imported_at.is_none() {
            download.imported_at = Some(self.clock.now().timestamp());
            self.repo.save(&mut download).await?;
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
