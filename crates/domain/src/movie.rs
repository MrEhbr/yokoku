use jiff::{SignedDuration, Timestamp, ToSpan, civil::Date};

use crate::{ExternalId, FileStatus, ItemFolder, MediaFileId, MovieId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Releases {
    pub cinema: Option<Date>,
    pub digital: Option<Date>,
    pub physical: Option<Date>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseKind {
    Cinema,
    Digital,
    Physical,
}

crate::string_enum!(ReleaseKind, "release kind" {
    Cinema => "cinema",
    Digital => "digital",
    Physical => "physical",
});

impl Releases {
    /// Known release dates, in kind order.
    pub fn dates(&self) -> impl Iterator<Item = (ReleaseKind, Date)> {
        [
            (ReleaseKind::Cinema, self.cinema),
            (ReleaseKind::Digital, self.digital),
            (ReleaseKind::Physical, self.physical),
        ]
        .into_iter()
        .filter_map(|(kind, date)| Some((kind, date?)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovieStatus {
    Announced,
    InCinemas,
    Released,
}

crate::string_enum!(MovieStatus, "movie status" {
    Announced => "announced",
    InCinemas => "in_cinemas",
    Released => "released",
});

/// A movie as its metadata source describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieMetadata {
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    /// Other names the item is known by, such as romanisations.
    pub alternate_titles: Vec<String>,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub releases: Releases,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    pub id: MovieId,
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    /// Other names the item is known by, such as romanisations.
    pub alternate_titles: Vec<String>,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub releases: Releases,
    /// Set when the movie is added; never changes.
    pub folder: ItemFolder,
    pub monitored: bool,
    pub file: Option<MediaFileId>,
    pub added_at: Timestamp,
    pub refreshed_at: Timestamp,
    /// Saves so far; storage refuses a save made from an older revision.
    pub revision: u64,
}

impl Movie {
    pub fn add(metadata: MovieMetadata, folder: ItemFolder, monitored: bool, now: Timestamp) -> Self {
        Self {
            id: MovieId::generate(),
            source: metadata.source,
            title: metadata.title,
            original_title: metadata.original_title,
            alternate_titles: metadata.alternate_titles,
            year: metadata.year,
            poster_path: metadata.poster_path,
            releases: metadata.releases,
            folder,
            monitored,
            file: None,
            added_at: now,
            refreshed_at: now,
            revision: 0,
        }
    }

    pub fn refresh(&mut self, metadata: MovieMetadata, now: Timestamp) {
        self.title = metadata.title;
        self.original_title = metadata.original_title;
        self.alternate_titles = metadata.alternate_titles;
        self.year = metadata.year;
        self.poster_path = metadata.poster_path;
        self.releases = metadata.releases;
        self.refreshed_at = now;
    }

    /// A release counts from its date onwards.
    pub fn status(&self, today: Date) -> MovieStatus {
        let reached = |date: Option<Date>| date.is_some_and(|date| date <= today);
        if reached(self.releases.digital) || reached(self.releases.physical) {
            MovieStatus::Released
        } else if reached(self.releases.cinema) {
            MovieStatus::InCinemas
        } else {
            MovieStatus::Announced
        }
    }

    /// Radarr's rules: refreshed over 180 days ago; else not within 12 hours of the last refresh,
    /// and not yet released or with a physical release in the last 30 days or later.
    pub fn needs_refresh(&self, now: Timestamp, today: Date) -> bool {
        let age = now.duration_since(self.refreshed_at);
        if age > SignedDuration::from_hours(180 * 24) {
            return true;
        }
        if age < SignedDuration::from_hours(12) {
            return false;
        }
        let recent = today.saturating_sub(30.days());
        self.status(today) != MovieStatus::Released || self.releases.physical.is_some_and(|date| date > recent)
    }

    /// Missing only once a digital or physical release is out.
    pub fn file_status(&self, today: Date) -> FileStatus {
        match self.file {
            Some(_) => FileStatus::Downloaded,
            None if self.status(today) == MovieStatus::Released => FileStatus::Missing,
            None => FileStatus::Upcoming,
        }
    }
}
