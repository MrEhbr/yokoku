pub mod calendar;
pub mod detail;
pub mod manage;

use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use yokoku_domain::ItemId;

#[cfg(feature = "server")]
use super::{Dep, Library};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    #[default]
    Series,
    Movie,
}

/// Series and movie lifecycle statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Continuing,
    OnBreak,
    Ended,
    Announced,
    InCinemas,
    Released,
}

/// Whether an episode or movie has its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileStatus {
    Downloaded,
    Missing,
    Upcoming,
}

/// How many of an item's files the Jellyfin user has watched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WatchState {
    Watched,
    InProgress,
    Unwatched,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sort {
    #[default]
    Title,
    /// Newest first.
    Added,
    /// Soonest first; items without an upcoming release last.
    NextRelease,
}

/// A movie's file, or a series' episode files, on disk and missing; missing counts only what is
/// monitored and released without a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCount {
    pub downloaded: usize,
    pub missing: usize,
}

/// One item of the library list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: ItemId,
    pub title: String,
    pub year: Option<i16>,
    pub status: Status,
    pub files: FileCount,
    pub next_release: Option<Date>,
    /// The poster's URL; `None` when the item has no poster.
    pub poster: Option<String>,
    /// The root folder holding the item's folder.
    pub root: String,
}

impl Kind {
    pub const ALL: [Self; 2] = [Self::Series, Self::Movie];

    pub fn label(self) -> &'static str {
        match self {
            Self::Series => "Series",
            Self::Movie => "Movie",
        }
    }
}

/// `series` or `movie`, as in route queries.
impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Series => "series",
            Self::Movie => "movie",
        })
    }
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "series" => Ok(Self::Series),
            "movie" => Ok(Self::Movie),
            _ => Err(format!("{value:?} is not series or movie")),
        }
    }
}

impl Status {
    pub const ALL: [Self; 6] =
        [Self::Continuing, Self::OnBreak, Self::Ended, Self::Announced, Self::InCinemas, Self::Released];

    pub fn label(self) -> &'static str {
        match self {
            Self::Continuing => "Continuing",
            Self::OnBreak => "On break",
            Self::Ended => "Ended",
            Self::Announced => "Announced",
            Self::InCinemas => "In cinemas",
            Self::Released => "Released",
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Self::Continuing | Self::OnBreak | Self::Ended => Kind::Series,
            Self::Announced | Self::InCinemas | Self::Released => Kind::Movie,
        }
    }
}

impl FileStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Downloaded => "Downloaded",
            Self::Missing => "Missing",
            Self::Upcoming => "Upcoming",
        }
    }
}

impl WatchState {
    pub const ALL: [Self; 3] = [Self::Watched, Self::InProgress, Self::Unwatched];

    pub fn label(self) -> &'static str {
        match self {
            Self::Watched => "Watched",
            Self::InProgress => "In progress",
            Self::Unwatched => "Unwatched",
        }
    }
}

impl Sort {
    pub const ALL: [Self; 3] = [Self::Title, Self::Added, Self::NextRelease];

    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Added => "Date added",
            Self::NextRelease => "Next release",
        }
    }
}

/// `watched`: `Unwatched` keeps every item not fully watched, items without files included.
#[get("/api/library?kind&status&watched&sort", library: Dep<Library>)]
pub async fn library(
    kind: Option<Kind>,
    status: Option<Status>,
    watched: Option<WatchState>,
    sort: Option<Sort>,
) -> Result<Vec<Entry>, ServerFnError> {
    server::library(&library, kind, status, watched, sort).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_core::{
        library::{LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus, artwork_name},
        media::RootKind,
    };
    use yokoku_domain::{ArtworkKind, MediaKind, MovieStatus, SeriesStatus};

    use super::{Entry, FileCount, FileStatus, Kind, Library, Sort, Status, WatchState};
    use crate::api::artwork;

    pub(super) async fn library(
        library: &Library,
        kind: Option<Kind>,
        status: Option<Status>,
        watched: Option<WatchState>,
        sort: Option<Sort>,
    ) -> Result<Vec<Entry>, ServerFnError> {
        let filter = LibraryFilter {
            kind: kind.map(Kind::into),
            status: status.map(Status::into),
            watched: watched.map(WatchState::into),
        };
        let entries = library.list(filter, sort.unwrap_or_default().into()).await.map_err(|error| {
            error!(%error, "listing the library failed");
            ServerFnError::new("The library could not be loaded")
        })?;
        Ok(entries.into_iter().map(Entry::from).collect())
    }

    impl From<Kind> for MediaKind {
        fn from(kind: Kind) -> Self {
            match kind {
                Kind::Series => Self::Series,
                Kind::Movie => Self::Movie,
            }
        }
    }

    impl From<MediaKind> for Kind {
        fn from(kind: MediaKind) -> Self {
            match kind {
                MediaKind::Series => Self::Series,
                MediaKind::Movie => Self::Movie,
            }
        }
    }

    impl From<Kind> for RootKind {
        fn from(kind: Kind) -> Self {
            match kind {
                Kind::Series => Self::Series,
                Kind::Movie => Self::Movies,
            }
        }
    }

    impl From<RootKind> for Kind {
        fn from(kind: RootKind) -> Self {
            match kind {
                RootKind::Series => Self::Series,
                RootKind::Movies => Self::Movie,
            }
        }
    }

    impl From<Status> for LibraryStatus {
        fn from(status: Status) -> Self {
            match status {
                Status::Continuing => Self::Series(SeriesStatus::Continuing),
                Status::OnBreak => Self::Series(SeriesStatus::OnBreak),
                Status::Ended => Self::Series(SeriesStatus::Ended),
                Status::Announced => Self::Movie(MovieStatus::Announced),
                Status::InCinemas => Self::Movie(MovieStatus::InCinemas),
                Status::Released => Self::Movie(MovieStatus::Released),
            }
        }
    }

    impl From<LibraryStatus> for Status {
        fn from(status: LibraryStatus) -> Self {
            match status {
                LibraryStatus::Series(SeriesStatus::Continuing) => Self::Continuing,
                LibraryStatus::Series(SeriesStatus::OnBreak) => Self::OnBreak,
                LibraryStatus::Series(SeriesStatus::Ended) => Self::Ended,
                LibraryStatus::Movie(MovieStatus::Announced) => Self::Announced,
                LibraryStatus::Movie(MovieStatus::InCinemas) => Self::InCinemas,
                LibraryStatus::Movie(MovieStatus::Released) => Self::Released,
            }
        }
    }

    impl From<yokoku_domain::FileStatus> for FileStatus {
        fn from(status: yokoku_domain::FileStatus) -> Self {
            match status {
                yokoku_domain::FileStatus::Downloaded => Self::Downloaded,
                yokoku_domain::FileStatus::Missing => Self::Missing,
                yokoku_domain::FileStatus::Upcoming => Self::Upcoming,
            }
        }
    }

    impl From<WatchState> for yokoku_core::library::WatchState {
        fn from(state: WatchState) -> Self {
            match state {
                WatchState::Watched => Self::Watched,
                WatchState::InProgress => Self::InProgress,
                WatchState::Unwatched => Self::Unwatched,
            }
        }
    }

    impl From<Sort> for LibrarySort {
        fn from(sort: Sort) -> Self {
            match sort {
                Sort::Title => Self::Title,
                Sort::Added => Self::Added,
                Sort::NextRelease => Self::NextRelease,
            }
        }
    }

    impl From<LibraryEntry> for Entry {
        fn from(entry: LibraryEntry) -> Self {
            let poster = entry
                .poster_path
                .as_deref()
                .and_then(artwork_name)
                .map(|name| artwork::url(entry.id, ArtworkKind::Poster, name));
            Self {
                poster,
                id: entry.id,
                title: entry.title,
                year: entry.year,
                status: entry.status.into(),
                files: FileCount { downloaded: entry.files.downloaded, missing: entry.files.missing },
                next_release: entry.next_release,
                root: entry.root.display().to_string(),
            }
        }
    }
}
