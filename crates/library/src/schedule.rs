use std::sync::Arc;

use jiff::{ToSpan, civil::Date};
use yokoku_domain::{Clock, EpisodeRef, ExternalId, FileStatus, ItemId, MovieId, ReleaseKind, SeriesId};

use crate::{
    LibraryError,
    catalog::Catalog,
    ports::{MovieRepo, SeriesRepo},
};

/// Release tracking over monitored items: calendar, upcoming and missing.
pub struct Schedule {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEntry {
    pub date: Date,
    pub item: ItemId,
    pub source: ExternalId,
    pub title: String,
    pub release: CalendarRelease,
    pub status: FileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarRelease {
    Episode { reference: EpisodeRef, title: String },
    Movie(ReleaseKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Missing {
    pub series: Vec<MissingSeries>,
    pub movies: Vec<MissingMovie>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingSeries {
    pub id: SeriesId,
    pub source: ExternalId,
    pub title: String,
    pub year: Option<i16>,
    pub episodes: Vec<MissingEpisode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingEpisode {
    pub reference: EpisodeRef,
    pub title: String,
    pub air_date: Date,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMovie {
    pub id: MovieId,
    pub source: ExternalId,
    pub title: String,
    pub year: Option<i16>,
}

impl Schedule {
    pub fn new(series: Arc<dyn SeriesRepo>, movies: Arc<dyn MovieRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { series, movies, clock }
    }

    pub fn today(&self) -> Date {
        self.clock.now().date()
    }

    /// Monitored episodes and movie releases dated `from` to `to`, both inclusive, ordered by date.
    pub async fn calendar(&self, from: Date, to: Date) -> Result<Vec<CalendarEntry>, LibraryError> {
        let today = self.today();
        let catalog = Catalog::load(self.series.as_ref(), self.movies.as_ref()).await?;
        let in_range = |date: Date| from <= date && date <= to;
        let mut entries = Vec::new();

        for series in &catalog.series {
            for (reference, episode) in series.monitored_episodes() {
                let Some(date) = episode.air_date.filter(|&date| in_range(date)) else { continue };
                entries.push(CalendarEntry {
                    date,
                    item: ItemId::Series(series.id),
                    source: series.source,
                    title: series.title.clone(),
                    release: CalendarRelease::Episode { reference, title: episode.title.clone() },
                    status: episode.file_status(today),
                });
            }
        }
        for movie in catalog.movies.iter().filter(|movie| movie.monitored) {
            for (kind, date) in movie.releases.dates().filter(|&(_, date)| in_range(date)) {
                entries.push(CalendarEntry {
                    date,
                    item: ItemId::Movie(movie.id),
                    source: movie.source,
                    title: movie.title.clone(),
                    release: CalendarRelease::Movie(kind),
                    status: movie.file_status(today),
                });
            }
        }

        entries.sort_by_cached_key(|entry| (entry.date, entry.title.to_lowercase(), release_order(&entry.release)));
        Ok(entries)
    }

    /// The calendar from today through `days` days ahead.
    pub async fn upcoming(&self, days: u16) -> Result<Vec<CalendarEntry>, LibraryError> {
        let today = self.today();
        self.calendar(today, today + i64::from(days).days()).await
    }

    /// Monitored episodes that aired without a file, grouped by series, and released monitored movies without one.
    pub async fn missing(&self) -> Result<Missing, LibraryError> {
        let today = self.today();
        let catalog = Catalog::load(self.series.as_ref(), self.movies.as_ref()).await?;

        let mut series: Vec<_> = catalog
            .series
            .iter()
            .filter_map(|series| {
                let episodes: Vec<_> = series
                    .monitored_episodes()
                    .filter(|(_, episode)| episode.file_status(today) == FileStatus::Missing)
                    .filter_map(|(reference, episode)| {
                        Some(MissingEpisode { reference, title: episode.title.clone(), air_date: episode.air_date? })
                    })
                    .collect();
                (!episodes.is_empty()).then(|| MissingSeries {
                    id: series.id,
                    source: series.source,
                    title: series.title.clone(),
                    year: series.year,
                    episodes,
                })
            })
            .collect();
        series.sort_by_key(|series| series.title.to_lowercase());

        let mut movies: Vec<_> = catalog
            .movies
            .iter()
            .filter(|movie| movie.monitored && movie.file_status(today) == FileStatus::Missing)
            .map(|movie| MissingMovie {
                id: movie.id,
                source: movie.source,
                title: movie.title.clone(),
                year: movie.year,
            })
            .collect();
        movies.sort_by_key(|movie| movie.title.to_lowercase());

        Ok(Missing { series, movies })
    }
}

/// Monday through Sunday of the week containing `date`.
pub fn week_of(date: Date) -> (Date, Date) {
    let monday = date - i64::from(date.weekday().to_monday_zero_offset()).days();
    (monday, monday + 6.days())
}

/// First through last day of the month containing `date`.
pub fn month_of(date: Date) -> (Date, Date) {
    (date.first_of_month(), date.last_of_month())
}

fn release_order(release: &CalendarRelease) -> (Option<EpisodeRef>, Option<ReleaseKind>) {
    match release {
        CalendarRelease::Episode { reference, .. } => (Some(*reference), None),
        CalendarRelease::Movie(kind) => (None, Some(*kind)),
    }
}
