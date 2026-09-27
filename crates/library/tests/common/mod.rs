#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use jiff::{
    SignedDuration, Zoned,
    civil::{Date, date},
};
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_domain::{
    Clock, EpisodeMetadata, ExternalId, MediaKind, MovieMetadata, Releases, SeasonMetadata, SeriesMetadata,
    SourceStatus,
};
use yokoku_events::{Event, EventLog, Publisher};
use yokoku_library::{
    Calendar, Library, MetadataService,
    ports::{FolderNames, MetadataError, MetadataProvider, SearchResult},
};
use yokoku_system::FileSpool;

pub const TODAY: Date = date(2026, 9, 26);

/// The root folder items are added to.
pub const ROOT: &str = "/library";

pub struct FixedClock(Mutex<Zoned>);

impl FixedClock {
    pub fn advance(&self, by: SignedDuration) {
        let mut now = self.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Zoned {
        self.0.lock().unwrap().clone()
    }
}

/// Names each folder after the item's source id.
pub struct SourceFolders;

impl FolderNames for SourceFolders {
    fn series_folder(&self, metadata: &SeriesMetadata) -> String {
        metadata.source.to_string()
    }

    fn movie_folder(&self, metadata: &MovieMetadata) -> String {
        metadata.source.to_string()
    }
}

/// Serves metadata registered by the test.
#[derive(Default)]
pub struct StaticMetadata {
    series: Mutex<HashMap<ExternalId, SeriesMetadata>>,
    movies: Mutex<HashMap<ExternalId, MovieMetadata>>,
}

impl StaticMetadata {
    pub fn put_series(&self, metadata: SeriesMetadata) {
        self.series.lock().unwrap().insert(metadata.source, metadata);
    }

    pub fn put_movie(&self, metadata: MovieMetadata) {
        self.movies.lock().unwrap().insert(metadata.source, metadata);
    }

    pub fn forget(&self, source: ExternalId) {
        self.series.lock().unwrap().remove(&source);
        self.movies.lock().unwrap().remove(&source);
    }
}

#[async_trait]
impl MetadataProvider for StaticMetadata {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MetadataError> {
        let query = query.to_lowercase();
        let series = self
            .series
            .lock()
            .unwrap()
            .values()
            .map(|m| result(MediaKind::Series, m.source, &m.title))
            .collect::<Vec<_>>();
        let movies = self
            .movies
            .lock()
            .unwrap()
            .values()
            .map(|m| result(MediaKind::Movie, m.source, &m.title))
            .collect::<Vec<_>>();
        let mut results: Vec<_> =
            series.into_iter().chain(movies).filter(|r| r.title.to_lowercase().contains(&query)).collect();
        results.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(results)
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        self.series.lock().unwrap().get(&source).cloned().ok_or(MetadataError::NotFound(source))
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        self.movies.lock().unwrap().get(&source).cloned().ok_or(MetadataError::NotFound(source))
    }
}

fn result(kind: MediaKind, source: ExternalId, title: &str) -> SearchResult {
    SearchResult { kind, source, title: title.into(), original_title: title.into(), year: None, poster_path: None }
}

/// Seasons as `(number, air dates)`; source ids are assigned in order.
pub fn series_metadata(
    source: u64,
    title: &str,
    status: SourceStatus,
    seasons: &[(u16, &[Option<Date>])],
) -> SeriesMetadata {
    let mut next_source_id = source * 1000;
    SeriesMetadata {
        source: ExternalId::Tmdb(source),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        poster_path: None,
        status,
        seasons: seasons
            .iter()
            .map(|&(number, dates)| SeasonMetadata {
                number,
                episodes: dates
                    .iter()
                    .zip(1..)
                    .map(|(&air_date, episode)| {
                        next_source_id += 1;
                        EpisodeMetadata {
                            source_id: next_source_id,
                            number: episode,
                            title: format!("Episode {episode}"),
                            air_date,
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn movie_metadata(source: u64, title: &str, releases: Releases) -> MovieMetadata {
    MovieMetadata {
        source: ExternalId::Tmdb(source),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year: None,
        poster_path: None,
        releases,
    }
}

pub struct App {
    _dir: TempDir,
    pub db: Database,
    pub clock: Arc<FixedClock>,
    pub provider: Arc<StaticMetadata>,
    pub library: Library,
    pub calendar: Calendar,
    pub metadata: MetadataService,
}

impl App {
    pub async fn new() -> Self {
        let db = Database::open_in_memory().await.unwrap();
        let clock = Arc::new(FixedClock(Mutex::new(TODAY.at(12, 0, 0, 0).in_tz("Europe/Berlin").unwrap())));
        let provider = Arc::new(StaticMetadata::default());
        let repo = Arc::new(db.clone());
        let dir = TempDir::new().unwrap();
        let events =
            Publisher::new(Arc::new(db.event_log()), Arc::new(FileSpool::new(dir.path().join("yokoku.spool"))));
        let library = Library::new(repo.clone(), repo.clone(), clock.clone(), events.clone());
        let calendar = Calendar::new(repo.clone(), repo.clone(), clock.clone());
        let metadata =
            MetadataService::new(repo.clone(), repo, provider.clone(), Arc::new(SourceFolders), clock.clone(), events);
        Self { _dir: dir, db, clock, provider, library, calendar, metadata }
    }

    pub async fn events(&self) -> Vec<Event> {
        let recorded = self.db.event_log().read_after(None, 100).await.unwrap();
        recorded.into_iter().map(|recorded| recorded.event).collect()
    }
}
