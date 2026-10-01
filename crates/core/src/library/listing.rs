use std::{cmp::Ordering, fmt};

use jiff::{Timestamp, civil::Date};
use yokoku_domain::{ExternalId, ItemId, MediaKind, Movie, MovieStatus, Series, SeriesStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryStatus {
    Series(SeriesStatus),
    Movie(MovieStatus),
}

impl fmt::Display for LibraryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Series(status) => status.fmt(f),
            Self::Movie(status) => status.fmt(f),
        }
    }
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
            poster_path: series.artwork.poster.clone(),
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
            poster_path: movie.artwork.poster.clone(),
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

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, ToSpan, civil::date};
    use rstest::rstest;
    use yokoku_domain::{ExternalId, ItemId, MediaKind, MovieId, MovieStatus, SeriesId, SeriesStatus};

    use super::{LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus};

    /// Added `hour` hours after the epoch, with its next release `days` from 2026-09-26.
    fn entry(title: &str, status: LibraryStatus, hour: i64, days: Option<i64>) -> LibraryEntry {
        let id = match status {
            LibraryStatus::Series(_) => ItemId::Series(SeriesId::generate()),
            LibraryStatus::Movie(_) => ItemId::Movie(MovieId::generate()),
        };
        LibraryEntry {
            id,
            source: ExternalId::Tmdb(1),
            title: title.into(),
            year: None,
            poster_path: None,
            status,
            has_files: false,
            added_at: Timestamp::UNIX_EPOCH + hour.hours(),
            next_release: days.map(|days| date(2026, 9, 26) + days.days()),
        }
    }

    fn entries() -> Vec<LibraryEntry> {
        let (continuing, ended) = (SeriesStatus::Continuing, SeriesStatus::Ended);
        vec![
            entry("beta", LibraryStatus::Series(continuing), 1, Some(7)),
            entry("Alpha", LibraryStatus::Movie(MovieStatus::Released), 2, None),
            entry("Gamma", LibraryStatus::Series(ended), 2, None),
            entry("delta", LibraryStatus::Movie(MovieStatus::Announced), 0, Some(7)),
        ]
    }

    #[rstest]
    #[case::title_ignoring_case(LibrarySort::Title, ["Alpha", "beta", "delta", "Gamma"])]
    #[case::newest_first_then_title(LibrarySort::Added, ["Alpha", "Gamma", "beta", "delta"])]
    #[case::soonest_first_then_title(LibrarySort::NextRelease, ["beta", "delta", "Alpha", "Gamma"])]
    fn sorts(#[case] sort: LibrarySort, #[case] expected: [&str; 4]) {
        let mut entries = entries();

        entries.sort_by(|a, b| sort.compare(a, b));

        assert_eq!(entries.iter().map(|entry| entry.title.as_str()).collect::<Vec<_>>(), expected);
    }

    #[rstest]
    #[case::everything(LibraryFilter::default(), &["beta", "Alpha", "Gamma", "delta"])]
    #[case::series(LibraryFilter { kind: Some(MediaKind::Series), status: None }, &["beta", "Gamma"])]
    #[case::movies(LibraryFilter { kind: Some(MediaKind::Movie), status: None }, &["Alpha", "delta"])]
    #[case::status(
        LibraryFilter { kind: None, status: Some(LibraryStatus::Series(SeriesStatus::Ended)) },
        &["Gamma"],
    )]
    #[case::kind_and_status(
        LibraryFilter { kind: Some(MediaKind::Movie), status: Some(LibraryStatus::Series(SeriesStatus::Ended)) },
        &[],
    )]
    fn filters(#[case] filter: LibraryFilter, #[case] expected: &[&str]) {
        let kept: Vec<_> =
            entries().into_iter().filter(|entry| filter.matches(entry)).map(|entry| entry.title).collect();

        assert_eq!(kept, expected);
    }
}
