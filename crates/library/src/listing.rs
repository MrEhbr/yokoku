use std::cmp::Ordering;

use jiff::{Timestamp, civil::Date};
use yokoku_domain::{ExternalId, ItemId, MediaKind, Movie, MovieStatus, Series, SeriesStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryStatus {
    Series(SeriesStatus),
    Movie(MovieStatus),
}

/// One row of the library list (FR-1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    pub id: ItemId,
    pub source: ExternalId,
    pub title: String,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub status: LibraryStatus,
    pub has_files: bool,
    pub added_at: Timestamp,
    pub next_release: Option<Date>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryFilter {
    pub kind: Option<MediaKind>,
    pub status: Option<LibraryStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibrarySort {
    #[default]
    Title,
    /// Newest first.
    Added,
    /// Soonest first; items without an upcoming release last.
    NextRelease,
}

impl LibraryEntry {
    pub(crate) fn from_series(series: &Series, today: Date) -> Self {
        Self {
            id: ItemId::Series(series.id),
            source: series.source,
            title: series.title.clone(),
            year: series.year,
            poster_path: series.poster_path.clone(),
            status: LibraryStatus::Series(series.status(today)),
            has_files: series.episodes().any(|episode| episode.file.is_some()),
            added_at: series.added_at,
            next_release: series.next_episode(today).and_then(|(_, episode)| episode.air_date),
        }
    }

    pub(crate) fn from_movie(movie: &Movie, today: Date) -> Self {
        let releases = [movie.releases.cinema, movie.releases.digital, movie.releases.physical];
        Self {
            id: ItemId::Movie(movie.id),
            source: movie.source,
            title: movie.title.clone(),
            year: movie.year,
            poster_path: movie.poster_path.clone(),
            status: LibraryStatus::Movie(movie.status(today)),
            has_files: movie.file.is_some(),
            added_at: movie.added_at,
            next_release: releases.into_iter().flatten().filter(|&date| date >= today).min(),
        }
    }
}

impl LibraryFilter {
    pub(crate) fn matches(self, entry: &LibraryEntry) -> bool {
        self.kind.is_none_or(|kind| kind == entry.id.kind()) && self.status.is_none_or(|status| status == entry.status)
    }
}

impl LibrarySort {
    pub(crate) fn compare(self, a: &LibraryEntry, b: &LibraryEntry) -> Ordering {
        let by_title =
            || a.title.chars().flat_map(char::to_lowercase).cmp(b.title.chars().flat_map(char::to_lowercase));
        match self {
            Self::Title => by_title(),
            Self::Added => b.added_at.cmp(&a.added_at).then_with(by_title),
            Self::NextRelease => match (a.next_release, b.next_release) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            }
            .then_with(by_title),
        }
    }
}
