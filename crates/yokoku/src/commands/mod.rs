pub mod calendar;
pub mod greet;
pub mod list;
pub mod missing;
pub mod monitor;
pub mod numbering;
pub mod remove;
pub mod show;
pub mod upcoming;

use anyhow::{Context, Result};
use clap::ValueEnum;
use yokoku_domain::{ExternalId, FileStatus, MovieStatus, ReleaseKind, SeriesStatus};
use yokoku_library::{ItemId, Library, LibraryStatus, ports::MediaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    Series,
    Movie,
}

/// Identifies a library item by type and source id.
#[derive(Debug, clap::Args)]
pub struct ItemArgs {
    /// Item type
    pub kind: Kind,

    /// Source id, e.g. `tmdb:1396`
    pub source: ExternalId,
}

impl ItemArgs {
    pub async fn resolve(&self, library: &Library) -> Result<ItemId> {
        let id = match self.kind {
            Kind::Series => library.find_series(self.source).await?.map(|series| ItemId::Series(series.id)),
            Kind::Movie => library.find_movie(self.source).await?.map(|movie| ItemId::Movie(movie.id)),
        };
        id.with_context(|| format!("{} {} is not in the library", kind_label(self.kind.into()), self.source))
    }
}

impl From<Kind> for MediaKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Series => Self::Series,
            Kind::Movie => Self::Movie,
        }
    }
}

pub fn kind_label(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Series => "series",
        MediaKind::Movie => "movie",
    }
}

pub fn status_label(status: LibraryStatus) -> &'static str {
    match status {
        LibraryStatus::Series(SeriesStatus::Continuing) => "continuing",
        LibraryStatus::Series(SeriesStatus::OnBreak) => "on break",
        LibraryStatus::Series(SeriesStatus::Ended) => "ended",
        LibraryStatus::Movie(MovieStatus::Announced) => "announced",
        LibraryStatus::Movie(MovieStatus::InCinemas) => "in cinemas",
        LibraryStatus::Movie(MovieStatus::Released) => "released",
    }
}

pub fn file_status_label(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Downloaded => "downloaded",
        FileStatus::Missing => "missing",
        FileStatus::Upcoming => "upcoming",
    }
}

pub fn release_label(kind: ReleaseKind) -> &'static str {
    match kind {
        ReleaseKind::Cinema => "cinema release",
        ReleaseKind::Digital => "digital release",
        ReleaseKind::Physical => "physical release",
    }
}

pub fn title_with_year(title: &str, year: Option<i16>) -> String {
    match year {
        Some(year) => format!("{title} ({year})"),
        None => title.to_owned(),
    }
}
