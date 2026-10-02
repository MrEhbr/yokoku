use std::{cmp::Ordering, collections::HashSet, fmt, path::PathBuf};

use jiff::{Timestamp, civil::Date};
use yokoku_domain::{ExternalId, FileStatus, ItemId, MediaFileId, MediaKind, Movie, MovieStatus, Series, SeriesStatus};

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
    pub files: FileCount,
    /// `None` without files.
    pub watched: Option<WatchState>,
    pub added_at: Timestamp,
    pub next_release: Option<Date>,
    /// The root folder holding the item's folder.
    pub root: PathBuf,
}

/// A movie's file, or a series' episode files, on disk and missing; missing counts only what is
/// monitored and released without a file, as on the Wanted page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileCount {
    pub downloaded: usize,
    pub missing: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryFilter {
    pub kind: Option<MediaKind>,
    pub status: Option<LibraryStatus>,
    /// `Unwatched` keeps every item not fully watched, items without files included.
    pub watched: Option<WatchState>,
}

/// How many of an item's files the media server's user has watched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchState {
    Unwatched,
    InProgress,
    Watched,
}

impl WatchState {
    /// `None` without files.
    pub(crate) fn of(files: impl IntoIterator<Item = MediaFileId>, watched: &HashSet<MediaFileId>) -> Option<Self> {
        let (all, seen) =
            files.into_iter().fold((0, 0), |(all, seen), file| (all + 1, seen + usize::from(watched.contains(&file))));
        match seen {
            _ if all == 0 => None,
            0 => Some(Self::Unwatched),
            seen if seen == all => Some(Self::Watched),
            _ => Some(Self::InProgress),
        }
    }
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
    pub(crate) fn from_series(series: &Series, watched: &HashSet<MediaFileId>, today: Date) -> Self {
        Self {
            id: ItemId::Series(series.id),
            source: series.source,
            title: series.title.clone(),
            year: series.year,
            poster_path: series.artwork.poster.clone(),
            status: LibraryStatus::Series(series.status(today)),
            files: FileCount {
                downloaded: series.episodes().filter(|episode| episode.file.is_some()).count(),
                missing: series
                    .monitored_episodes()
                    .filter(|(_, episode)| episode.file_status(today) == FileStatus::Missing)
                    .count(),
            },
            watched: WatchState::of(series.episodes().filter_map(|episode| episode.file), watched),
            added_at: series.added_at,
            next_release: series.next_episode(today).and_then(|(_, episode)| episode.air_date),
            root: series.folder.root.clone(),
        }
    }

    pub(crate) fn from_movie(movie: &Movie, watched: &HashSet<MediaFileId>, today: Date) -> Self {
        let releases = [movie.releases.cinema, movie.releases.digital, movie.releases.physical];
        Self {
            id: ItemId::Movie(movie.id),
            source: movie.source,
            title: movie.title.clone(),
            year: movie.year,
            poster_path: movie.artwork.poster.clone(),
            status: LibraryStatus::Movie(movie.status(today)),
            files: FileCount {
                downloaded: usize::from(movie.file.is_some()),
                missing: usize::from(movie.monitored && movie.file_status(today) == FileStatus::Missing),
            },
            watched: WatchState::of(movie.file, watched),
            added_at: movie.added_at,
            next_release: releases.into_iter().flatten().filter(|&date| date >= today).min(),
            root: movie.folder.root.clone(),
        }
    }
}

impl LibraryFilter {
    pub(crate) fn matches(self, entry: &LibraryEntry) -> bool {
        self.kind.is_none_or(|kind| kind == entry.id.kind())
            && self.status.is_none_or(|status| status == entry.status)
            && self.watched.is_none_or(|watched| match watched {
                WatchState::Unwatched => entry.watched != Some(WatchState::Watched),
                watched => entry.watched == Some(watched),
            })
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
    use std::collections::HashSet;

    use jiff::{Timestamp, ToSpan, civil::date};
    use rstest::rstest;
    use yokoku_domain::{ExternalId, ItemId, MediaFileId, MediaKind, MovieId, MovieStatus, SeriesId, SeriesStatus};

    use super::{FileCount, LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus, WatchState};

    /// Added `hour` hours after the epoch, with its next release `days` from 2026-09-26.
    fn entry(
        title: &str,
        status: LibraryStatus,
        watched: Option<WatchState>,
        hour: i64,
        days: Option<i64>,
    ) -> LibraryEntry {
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
            files: FileCount::default(),
            watched,
            added_at: Timestamp::UNIX_EPOCH + hour.hours(),
            next_release: days.map(|days| date(2026, 9, 26) + days.days()),
            root: "/media".into(),
        }
    }

    fn entries() -> Vec<LibraryEntry> {
        let (continuing, ended) = (SeriesStatus::Continuing, SeriesStatus::Ended);
        vec![
            entry("beta", LibraryStatus::Series(continuing), Some(WatchState::InProgress), 1, Some(7)),
            entry("Alpha", LibraryStatus::Movie(MovieStatus::Released), Some(WatchState::Watched), 2, None),
            entry("Gamma", LibraryStatus::Series(ended), Some(WatchState::Unwatched), 2, None),
            entry("delta", LibraryStatus::Movie(MovieStatus::Announced), None, 0, Some(7)),
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
    #[case::series(LibraryFilter { kind: Some(MediaKind::Series), ..LibraryFilter::default() }, &["beta", "Gamma"])]
    #[case::movies(LibraryFilter { kind: Some(MediaKind::Movie), ..LibraryFilter::default() }, &["Alpha", "delta"])]
    #[case::status(
        LibraryFilter { status: Some(LibraryStatus::Series(SeriesStatus::Ended)), ..LibraryFilter::default() },
        &["Gamma"],
    )]
    #[case::kind_and_status(
        LibraryFilter {
            kind: Some(MediaKind::Movie),
            status: Some(LibraryStatus::Series(SeriesStatus::Ended)),
            watched: None,
        },
        &[],
    )]
    #[case::watched(LibraryFilter { watched: Some(WatchState::Watched), ..LibraryFilter::default() }, &["Alpha"])]
    #[case::in_progress(
        LibraryFilter { watched: Some(WatchState::InProgress), ..LibraryFilter::default() },
        &["beta"],
    )]
    #[case::unwatched_is_everything_not_fully_watched(
        LibraryFilter { watched: Some(WatchState::Unwatched), ..LibraryFilter::default() },
        &["beta", "Gamma", "delta"],
    )]
    fn filters(#[case] filter: LibraryFilter, #[case] expected: &[&str]) {
        let kept: Vec<_> =
            entries().into_iter().filter(|entry| filter.matches(entry)).map(|entry| entry.title).collect();

        assert_eq!(kept, expected);
    }

    /// `files` and `watched` index the same three file ids.
    #[rstest]
    #[case::no_files(&[], &[], None)]
    #[case::none_watched(&[0, 1], &[2], Some(WatchState::Unwatched))]
    #[case::some_watched(&[0, 1], &[1, 2], Some(WatchState::InProgress))]
    #[case::all_watched(&[0, 1], &[0, 1], Some(WatchState::Watched))]
    #[case::a_file_of_two_episodes_counts_for_each(&[0, 0, 1], &[0], Some(WatchState::InProgress))]
    fn watch_state(#[case] files: &[usize], #[case] watched: &[usize], #[case] expected: Option<WatchState>) {
        let ids = [MediaFileId::generate(), MediaFileId::generate(), MediaFileId::generate()];
        let watched: HashSet<_> = watched.iter().map(|&n| ids[n]).collect();

        assert_eq!(WatchState::of(files.iter().map(|&n| ids[n]), &watched), expected);
    }
}
