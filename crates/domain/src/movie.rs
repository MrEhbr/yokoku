use jiff::{Timestamp, civil::Date};

use crate::{ExternalId, FileStatus, MovieId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Releases {
    pub cinema: Option<Date>,
    pub digital: Option<Date>,
    pub physical: Option<Date>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovieStatus {
    Announced,
    InCinemas,
    Released,
}

/// A movie as its metadata source describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieMetadata {
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
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
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub releases: Releases,
    pub monitored: bool,
    pub has_file: bool,
    pub added_at: Timestamp,
    pub refreshed_at: Timestamp,
}

impl Movie {
    pub fn add(metadata: MovieMetadata, monitored: bool, now: Timestamp) -> Self {
        Self {
            id: MovieId::generate(),
            source: metadata.source,
            title: metadata.title,
            original_title: metadata.original_title,
            year: metadata.year,
            poster_path: metadata.poster_path,
            releases: metadata.releases,
            monitored,
            has_file: false,
            added_at: now,
            refreshed_at: now,
        }
    }

    pub fn refresh(&mut self, metadata: MovieMetadata, now: Timestamp) {
        self.title = metadata.title;
        self.original_title = metadata.original_title;
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

    /// Missing only once a digital or physical release is out.
    pub fn file_status(&self, today: Date) -> FileStatus {
        if self.has_file {
            FileStatus::Downloaded
        } else if self.status(today) == MovieStatus::Released {
            FileStatus::Missing
        } else {
            FileStatus::Upcoming
        }
    }
}
