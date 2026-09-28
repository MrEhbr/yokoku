use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use yokoku_domain::ItemId;

#[cfg(feature = "server")]
use super::{Dep, Library};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Series,
    Movie,
}

/// Series and movie lifecycle statuses (FR-1.2).
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

/// One item of the library list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: ItemId,
    pub title: String,
    pub year: Option<i16>,
    pub status: Status,
    pub has_files: bool,
    pub next_release: Option<Date>,
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

#[get("/api/library?kind&status&sort", library: Dep<Library>)]
pub async fn library(
    kind: Option<Kind>,
    status: Option<Status>,
    sort: Option<Sort>,
) -> Result<Vec<Entry>, ServerFnError> {
    server::library(&library, kind, status, sort).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_domain::{MediaKind, MovieStatus, SeriesStatus};
    use yokoku_library::{LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus};

    use super::{Entry, Kind, Library, Sort, Status};

    pub(super) async fn library(
        library: &Library,
        kind: Option<Kind>,
        status: Option<Status>,
        sort: Option<Sort>,
    ) -> Result<Vec<Entry>, ServerFnError> {
        let filter = LibraryFilter { kind: kind.map(Kind::into), status: status.map(Status::into) };
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
            Self {
                id: entry.id,
                title: entry.title,
                year: entry.year,
                status: entry.status.into(),
                has_files: entry.has_files,
                next_release: entry.next_release,
            }
        }
    }
}
