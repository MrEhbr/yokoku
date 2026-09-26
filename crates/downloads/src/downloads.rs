use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use yokoku_domain::{Clock, DownloadId, ItemId};
use yokoku_events::{Event, HandlerError, Recorded, Subscriber};

use crate::{
    Download, DownloadError, DownloadState, DownloadStatus,
    ports::{DownloadClient, DownloadRepo, StorageError, Torrent, TorrentSource},
};

/// Torrents added through Yokoku and their state in the download client (FR-3).
pub struct Downloads {
    repo: Arc<dyn DownloadRepo>,
    client: Arc<dyn DownloadClient>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub synced: usize,
    /// Downloads that finished since the last sync.
    pub completed: Vec<DownloadId>,
    /// Downloads the client no longer has.
    pub removed: usize,
}

impl Downloads {
    pub fn new(repo: Arc<dyn DownloadRepo>, client: Arc<dyn DownloadClient>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, client, clock }
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

        let mut download = Download {
            id: DownloadId::generate(),
            hash: added.hash,
            name: added.name,
            item,
            status: DownloadStatus::unknown(),
            added_at: self.clock.now().timestamp(),
            completed_at: None,
            imported_at: None,
            revision: 0,
        };
        let torrent = self.client.torrents(std::slice::from_ref(&download.hash)).await?.pop();
        let mut events = vec![Event::TorrentAdded { download: download.id, name: download.name.clone(), item }];
        events.extend(self.apply(&mut download, torrent));
        self.repo.save(&mut download, &events).await?;
        Ok(download)
    }

    /// Records each download's progress and state; a download that finished emits
    /// `DownloadCompleted` once, however often and however many syncs run. A download another
    /// sync saved in the meantime is left to that sync.
    pub async fn sync(&self) -> Result<SyncReport, DownloadError> {
        let active: Vec<Download> = self
            .repo
            .list()
            .await?
            .into_iter()
            .filter(|download| download.status.state != DownloadState::Removed)
            .collect();
        let hashes: Vec<String> = active.iter().map(|download| download.hash.clone()).collect();
        let mut torrents: HashMap<String, Torrent> =
            self.client.torrents(&hashes).await?.into_iter().map(|torrent| (torrent.hash.clone(), torrent)).collect();

        let mut report = SyncReport::default();
        for mut download in active {
            let torrent = torrents.remove(&download.hash);
            let removed = torrent.is_none();
            let completed = self.apply(&mut download, torrent);
            match self.repo.save(&mut download, completed.as_slice()).await {
                Err(StorageError::Conflict) => continue,
                result => result?,
            }
            report.synced += 1;
            report.removed += usize::from(removed);
            if completed.is_some() {
                report.completed.push(download.id);
            }
        }
        Ok(report)
    }

    /// Records that files from the download reached the library; the first import counts.
    pub async fn mark_imported(&self, id: DownloadId) -> Result<(), DownloadError> {
        let Some(mut download) = self.repo.get(id).await? else { return Ok(()) };
        if download.imported_at.is_none() {
            download.imported_at = Some(self.clock.now().timestamp());
            self.repo.save(&mut download, &[]).await?;
        }
        Ok(())
    }

    /// Returns `DownloadCompleted` when the download has just finished.
    fn apply(&self, download: &mut Download, torrent: Option<Torrent>) -> Option<Event> {
        let Some(torrent) = torrent else {
            download.status = DownloadStatus {
                state: DownloadState::Removed,
                download_rate: 0,
                eta: None,
                ..download.status.clone()
            };
            return None;
        };
        download.name = torrent.name;
        download.status = torrent.status;
        if !torrent.complete || download.completed_at.is_some() {
            return None;
        }
        download.completed_at = Some(self.clock.now().timestamp());
        Some(Event::DownloadCompleted {
            download: download.id,
            name: download.name.clone(),
            content_path: download.content_path(),
            item: download.item,
        })
    }
}

#[async_trait]
impl Subscriber for Downloads {
    fn name(&self) -> &'static str {
        "downloads.imports"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        if let Event::FilesImported { download: Some(download), .. } = &recorded.event {
            self.mark_imported(*download).await?;
        }
        Ok(())
    }
}
