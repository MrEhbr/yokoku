//! Downloads as of the last sync with the download client, each with its import (FR-3.4, 4.11,
//! 9.2).

use dioxus::{fullstack::ServerEvents, prelude::*};
use serde::{Deserialize, Serialize};
use yokoku_domain::{DownloadId, ImportId, ItemId};

#[cfg(feature = "server")]
use crate::api::{Dep, Downloads, Importer, Library, QueueChanges, Reviewer};

/// A series or movie in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemLink {
    pub id: ItemId,
    pub title: String,
}

/// Newest first; torrents no longer in the client are left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub id: DownloadId,
    pub name: String,
    pub item: Option<ItemLink>,
    pub state: DownloadState,
    /// 0 to 100.
    pub percent: u8,
    /// Bytes.
    pub size: u64,
    /// Bytes per second.
    pub rate: u64,
    /// Seconds until done, when the client can tell.
    pub eta: Option<u64>,
    /// Its import while not done.
    pub import: Option<ImportEntry>,
    /// Its files reached the library.
    pub imported: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadState {
    Queued,
    Checking,
    Downloading,
    Seeding,
    /// Stopped after it completed.
    Finished,
    Paused,
    /// The client reports an error.
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportEntry {
    pub id: ImportId,
    pub state: ImportState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportState {
    NeedsReview,
    Failed(String),
    Queued,
    Importing,
}

impl DownloadState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Checking => "Checking",
            Self::Downloading => "Downloading",
            Self::Seeding => "Seeding",
            Self::Finished => "Finished",
            Self::Paused => "Paused",
            Self::Error(_) => "Error",
        }
    }
}

#[get("/api/downloads", downloads: Dep<Downloads>, reviewer: Dep<Reviewer>, importer: Dep<Importer>, library: Dep<Library>)]
pub async fn downloads() -> Result<Vec<DownloadEntry>, ServerFnError> {
    server::downloads(&downloads, &reviewer, &importer, &library).await
}

/// The downloads at once, then again each time they or their imports change.
#[get(
    "/api/downloads/live",
    downloads: Dep<Downloads>,
    reviewer: Dep<Reviewer>,
    importer: Dep<Importer>,
    library: Dep<Library>,
    changes: Dep<QueueChanges>
)]
pub async fn live_downloads() -> Result<ServerEvents<Vec<DownloadEntry>>, ServerFnError> {
    let sources = server::Sources {
        downloads: downloads.into_inner(),
        reviewer: reviewer.into_inner(),
        importer: importer.into_inner(),
        library: library.into_inner(),
    };
    Ok(server::live(sources, changes.into_inner()))
}

/// Queues a failed import again; the import job carries it out.
#[post("/api/imports/{id}/retry", importer: Dep<Importer>)]
pub async fn retry_import(id: ImportId) -> Result<(), ServerFnError> {
    server::retry(&importer, id).await
}

#[cfg(feature = "server")]
mod server {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use dioxus::{fullstack::ServerEvents, logger::tracing::error, prelude::*};
    use yokoku_domain::{DownloadId, ImportId, ItemId};
    use yokoku_downloads::Download;
    use yokoku_library::{LibraryFilter, LibrarySort};
    use yokoku_media::{Import, ImportStatus, MediaError};

    use super::{
        DownloadEntry, DownloadState, Downloads, ImportEntry, ImportState, Importer, ItemLink, Library, QueueChanges,
        Reviewer,
    };

    /// How long a burst of saves settles before the downloads are read again.
    const SETTLE: Duration = Duration::from_millis(250);
    /// Reads again this often without a change signal, for changes made by other processes.
    const FALLBACK: Duration = Duration::from_secs(30);

    pub(super) struct Sources {
        pub downloads: Arc<Downloads>,
        pub reviewer: Arc<Reviewer>,
        pub importer: Arc<Importer>,
        pub library: Arc<Library>,
    }

    /// Sends the downloads, then waits for a change and sends them again when they differ from
    /// the last ones sent; ends once the client is gone.
    pub(super) fn live(sources: Sources, changes: Arc<QueueChanges>) -> ServerEvents<Vec<DownloadEntry>> {
        ServerEvents::new(move |mut tx| async move {
            let mut watch = changes.watch();
            let mut sent = None;
            loop {
                watch.borrow_and_update();
                let read = downloads(&sources.downloads, &sources.reviewer, &sources.importer, &sources.library);
                if let Ok(current) = read.await
                    && sent.as_ref() != Some(&current)
                {
                    if tx.send(current.clone()).await.is_err() {
                        break;
                    }
                    sent = Some(current);
                }
                tokio::select! {
                    changed = watch.changed() => if changed.is_err() { break },
                    () = tokio::time::sleep(FALLBACK) => {},
                }
                tokio::time::sleep(SETTLE).await;
            }
        })
    }

    pub(super) async fn downloads(
        downloads: &Downloads,
        reviewer: &Reviewer,
        importer: &Importer,
        library: &Library,
    ) -> Result<Vec<DownloadEntry>, ServerFnError> {
        let failed = |error: &dyn std::fmt::Display| {
            error!(%error, "loading downloads and their imports failed");
            ServerFnError::new("Downloads could not be loaded")
        };
        let listed = downloads.list().await.map_err(|error| failed(&error))?;
        let mut imports = reviewer.pending().await.map_err(|error| failed(&error))?;
        imports.extend(importer.list().await.map_err(|error| failed(&error))?);
        let imports: HashMap<DownloadId, Import> =
            imports.into_iter().filter_map(|import| Some((import.download?, import))).collect();
        let items = library.list(LibraryFilter::default(), LibrarySort::Title).await.map_err(|error| failed(&error))?;
        let titles: HashMap<ItemId, String> = items.into_iter().map(|entry| (entry.id, entry.title)).collect();

        Ok(listed
            .into_iter()
            .filter(|download| download.status.state != yokoku_downloads::DownloadState::Removed)
            .map(|download| {
                let item =
                    download.item.and_then(|id| titles.get(&id).map(|title| ItemLink { id, title: title.clone() }));
                let import = imports.get(&download.id).map(ImportEntry::from);
                entry(download, item, import)
            })
            .collect())
    }

    pub(super) async fn retry(importer: &Importer, id: ImportId) -> Result<(), ServerFnError> {
        importer.retry(id).await.map_err(|error| match error {
            MediaError::ImportNotFound(_) | MediaError::NotFailed(_) => {
                ServerFnError::new("The import is no longer waiting for a retry")
            },
            error => {
                error!(%error, import = %id, "retrying the import failed");
                ServerFnError::new("The import could not be retried")
            },
        })
    }

    fn entry(download: Download, item: Option<ItemLink>, import: Option<ImportEntry>) -> DownloadEntry {
        use yokoku_downloads::DownloadState as Client;
        let status = &download.status;
        let state = match (&status.error, status.state) {
            (Some(error), _) => DownloadState::Error(error.clone()),
            (None, Client::Queued) => DownloadState::Queued,
            (None, Client::Checking) => DownloadState::Checking,
            (None, Client::Downloading) => DownloadState::Downloading,
            (None, Client::Seeding) => DownloadState::Seeding,
            (None, Client::Stopped | Client::Removed) if download.completed_at.is_some() => DownloadState::Finished,
            (None, Client::Stopped | Client::Removed) => DownloadState::Paused,
        };
        DownloadEntry {
            id: download.id,
            percent: download.percent_done(),
            size: status.size,
            rate: status.download_rate,
            eta: status.eta,
            imported: download.imported_at.is_some(),
            name: download.name,
            item,
            state,
            import,
        }
    }

    impl From<&Import> for ImportEntry {
        fn from(import: &Import) -> Self {
            let state = match import.status {
                ImportStatus::NeedsReview => ImportState::NeedsReview,
                ImportStatus::Failed => ImportState::Failed(import.error.clone().unwrap_or_default()),
                ImportStatus::Approved | ImportStatus::Done => ImportState::Queued,
                ImportStatus::Importing => ImportState::Importing,
            };
            Self { id: import.id, state }
        }
    }
}
