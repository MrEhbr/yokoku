//! What happened in the library, newest first (FR-9.1).

use dioxus::prelude::*;
use jiff::civil::{Date, DateTime};
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MovieId, SeriesId};

#[cfg(feature = "server")]
use crate::api::{Clock, Dep, History, Library};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryPage {
    pub today: Date,
    pub entries: Vec<HistoryEntry>,
    /// The `before` that loads the next page; `None` on the last one.
    pub older: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    /// In the configured time zone.
    pub at: DateTime,
    pub summary: Vec<Part>,
    /// File paths, one line per file.
    pub details: Vec<Vec<Part>>,
}

/// A piece of an entry's text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Part {
    Text(String),
    /// A series or movie; `id` is `None` once it is no longer in the library.
    Item {
        id: Option<ItemId>,
        title: String,
    },
    /// Episodes, like `S01E01-E03`, or a setting's key.
    Code(String),
    Path(String),
}

#[get("/api/history?before", history: Dep<History>, library: Dep<Library>, clock: Dep<dyn Clock>)]
pub async fn history(before: Option<i64>) -> Result<HistoryPage, ServerFnError> {
    server::page(&history, &library, &*clock, None, before, server::PAGE).await
}

#[get("/api/series/{id}/history?before", history: Dep<History>, library: Dep<Library>, clock: Dep<dyn Clock>)]
pub async fn series_history(id: SeriesId, before: Option<i64>) -> Result<HistoryPage, ServerFnError> {
    server::page(&history, &library, &*clock, Some(ItemId::Series(id)), before, server::ITEM_PAGE).await
}

#[get("/api/movies/{id}/history?before", history: Dep<History>, library: Dep<Library>, clock: Dep<dyn Clock>)]
pub async fn movie_history(id: MovieId, before: Option<i64>) -> Result<HistoryPage, ServerFnError> {
    server::page(&history, &library, &*clock, Some(ItemId::Movie(id)), before, server::ITEM_PAGE).await
}

#[cfg(feature = "server")]
mod server {
    use std::{collections::HashMap, path::Path};

    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_core::{
        events::{
            DeleteReason, DownloadCompleted, EpisodesRenumbered, Event, EventId, FileDeleted, FileRenamed, FilesFound,
            FilesImported, ImportFailed, ImportNeedsReview, LinkedFile, MovieAdded, MovieRemoved, SeriesAdded,
            SeriesRemoved, SettingsChanged, TorrentAdded, TorrentRemoved,
        },
        library::{LibraryFilter, LibrarySort},
    };
    use yokoku_domain::{FileTarget, ItemId};

    use super::{Clock, History, HistoryEntry, HistoryPage, Library, Part};

    /// Entries of the whole library on the History page.
    pub(super) const PAGE: usize = 50;
    /// Entries of one item on its detail page.
    pub(super) const ITEM_PAGE: usize = 10;

    pub(super) async fn page(
        history: &History,
        library: &Library,
        clock: &dyn Clock,
        item: Option<ItemId>,
        before: Option<i64>,
        limit: usize,
    ) -> Result<HistoryPage, ServerFnError> {
        let failed = |error: &dyn std::fmt::Display| {
            error!(%error, "reading the history failed");
            ServerFnError::new("The history could not be loaded")
        };
        let mut recorded = history.page(item, before.map(EventId), limit + 1).await.map_err(|error| failed(&error))?;
        let older = (recorded.len() > limit).then(|| {
            recorded.truncate(limit);
            recorded.last().map_or(0, |last| last.id.0)
        });
        let listed =
            library.list(LibraryFilter::default(), LibrarySort::Title).await.map_err(|error| failed(&error))?;
        let wording = Wording { titles: listed.into_iter().map(|entry| (entry.id, entry.title)).collect(), here: item };
        let now = clock.now();
        let entries = recorded
            .into_iter()
            .map(|recorded| {
                let (mut summary, details) = wording.event(&recorded.event);
                summary.retain(|part| *part != Part::Text(String::new()));
                HistoryEntry {
                    id: recorded.id.0,
                    at: recorded.occurred_at.to_zoned(now.time_zone().clone()).datetime(),
                    summary,
                    details,
                }
            })
            .collect();
        Ok(HistoryPage { today: now.date(), entries, older })
    }

    /// Words events for the web: items by title, files by the episodes they hold, paths in the
    /// details.
    struct Wording {
        titles: HashMap<ItemId, String>,
        /// The item whose page this is; it goes unnamed where the text only says what an event
        /// concerns.
        here: Option<ItemId>,
    }

    type Lines = (Vec<Part>, Vec<Vec<Part>>);

    fn text(text: impl Into<String>) -> Part {
        Part::Text(text.into())
    }

    fn path(path: &Path) -> Part {
        Part::Path(path.display().to_string())
    }

    impl Wording {
        fn event(&self, event: &Event) -> Lines {
            let with_download = |name: &str, item: &Option<ItemId>, verb: &str, after: &str| {
                let mut summary = vec![text(format!("{verb} {name}"))];
                if let Some(item) = item {
                    summary.extend(self.about(" for ", *item));
                }
                summary.push(text(after));
                (summary, Vec::new())
            };
            match event {
                Event::SeriesAdded(SeriesAdded { series, title }) => {
                    (vec![text("Added series "), self.item(ItemId::Series(*series), Some(title))], Vec::new())
                },
                Event::MovieAdded(MovieAdded { movie, title }) => {
                    (vec![text("Added movie "), self.item(ItemId::Movie(*movie), Some(title))], Vec::new())
                },
                Event::SeriesRemoved(SeriesRemoved { series, title, delete_files }) => (
                    vec![
                        text("Removed series "),
                        self.item(ItemId::Series(*series), Some(title)),
                        text(if *delete_files { " and its files" } else { "" }),
                    ],
                    Vec::new(),
                ),
                Event::MovieRemoved(MovieRemoved { movie, title, delete_files }) => (
                    vec![
                        text("Removed movie "),
                        self.item(ItemId::Movie(*movie), Some(title)),
                        text(if *delete_files { " and its files" } else { "" }),
                    ],
                    Vec::new(),
                ),
                Event::FilesFound(FilesFound { files }) => self.files("Found", files),
                Event::FilesImported(FilesImported { files, .. }) => self.files("Imported", files),
                Event::FileDeleted(FileDeleted { path: deleted, target, reason, .. }) => {
                    let reason = match reason {
                        DeleteReason::External => "gone from disk",
                        DeleteReason::Replaced => "replaced by an import",
                        DeleteReason::User => "by request",
                        DeleteReason::ItemRemoved => "its item was removed",
                    };
                    let mut summary = vec![text("Deleted ")];
                    summary.extend(self.target(target));
                    summary.push(text(format!(" ({reason})")));
                    (summary, vec![vec![path(deleted)]])
                },
                Event::FileRenamed(FileRenamed { from, to, target, .. }) => {
                    let mut summary = vec![text("Renamed ")];
                    match target {
                        Some(target) => summary.extend(self.target(target)),
                        None => summary.push(text("a file")),
                    }
                    (summary, vec![vec![path(from), text(" → "), path(to)]])
                },
                Event::EpisodesRenumbered(EpisodesRenumbered { series, files }) => {
                    let files = if files.len() == 1 { "1 file".to_owned() } else { format!("{} files", files.len()) };
                    let mut summary = vec![text(format!("Renumbered the episodes of {files}"))];
                    summary.extend(self.about(" of ", ItemId::Series(*series)));
                    (summary, Vec::new())
                },
                Event::ImportNeedsReview(ImportNeedsReview { source, .. }) => {
                    (vec![text("An import needs review")], vec![vec![path(source)]])
                },
                Event::ImportFailed(ImportFailed { source, reason, .. }) => {
                    (vec![text(format!("An import failed: {reason}"))], vec![vec![path(source)]])
                },
                Event::TorrentAdded(TorrentAdded { name, item, .. }) => with_download(name, item, "Added torrent", ""),
                Event::DownloadCompleted(DownloadCompleted { name, item, .. }) => {
                    with_download(name, item, "Finished downloading", "")
                },
                Event::TorrentRemoved(TorrentRemoved { name, item, .. }) => {
                    with_download(name, item, "Removed torrent", " after seeding")
                },
                Event::SettingsChanged(SettingsChanged { key }) => {
                    (vec![text("Changed setting "), Part::Code(key.clone())], Vec::new())
                },
            }
        }

        /// Linked while in the library; `title` names it once removed.
        fn item(&self, id: ItemId, title: Option<&str>) -> Part {
            match (self.titles.get(&id), title) {
                (Some(listed), _) => Part::Item { id: Some(id), title: listed.clone() },
                (None, Some(title)) => Part::Item { id: None, title: title.to_owned() },
                (None, None) => match id {
                    ItemId::Series(_) => text("a removed series"),
                    ItemId::Movie(_) => text("a removed movie"),
                },
            }
        }

        /// ` of Frieren`; nothing on Frieren's own page.
        fn about(&self, connector: &str, id: ItemId) -> Vec<Part> {
            if self.here == Some(id) { Vec::new() } else { vec![text(connector), self.item(id, None)] }
        }

        /// `S01E01 of Frieren`, or `the file of Dune`.
        fn target(&self, target: &FileTarget) -> Vec<Part> {
            let mut parts = match target {
                FileTarget::Episodes { span, .. } => vec![Part::Code(span.to_string())],
                FileTarget::Movie(_) => vec![text("the file")],
            };
            parts.extend(self.about(" of ", target.item()));
            parts
        }

        fn files(&self, verb: &str, files: &[LinkedFile]) -> Lines {
            if let [file] = files {
                let mut summary = vec![text(format!("{verb} "))];
                summary.extend(self.target(&file.target));
                return (summary, vec![vec![path(&file.path)]]);
            }
            let first = files.first().map(|file| file.target.item());
            let one_item = first.filter(|first| files.iter().all(|file| file.target.item() == *first));
            let mut summary = vec![text(format!("{verb} {} files", files.len()))];
            if let Some(item) = one_item {
                summary.extend(self.about(" of ", item));
            }
            let details = files
                .iter()
                .map(|file| {
                    let mut line = Vec::new();
                    if one_item.is_none() {
                        line.extend([self.item(file.target.item(), None), text(" ")]);
                    }
                    if let FileTarget::Episodes { span, .. } = &file.target {
                        line.extend([Part::Code(span.to_string()), text(" ")]);
                    }
                    line.push(path(&file.path));
                    line
                })
                .collect();
            (summary, details)
        }
    }
}
