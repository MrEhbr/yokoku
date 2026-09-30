#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use tempfile::TempDir;
use yokoku_core::{
    events::EventLog,
    library::{
        Calendar, Library, MetadataService,
        ports::{FolderNames, MetadataError, MetadataProvider, SearchResult},
    },
};
use yokoku_domain::{ExternalId, MediaKind, MovieMetadata, SeriesMetadata, events::Event, title_with_year};
use yokoku_infra::db::Database;
pub use yokoku_test_support::{
    clock::TODAY,
    metadata::{movie_metadata, series_metadata},
};
use yokoku_test_support::{clock::TestClock, events::publisher};

/// The root folder items are added to.
pub const ROOT: &str = "/library";

/// Names each folder `Title (Year)`.
pub struct TitleFolders;

impl FolderNames for TitleFolders {
    fn series_folder(&self, title: &str, year: Option<i16>) -> String {
        title_with_year(title, year)
    }

    fn movie_folder(&self, title: &str, year: Option<i16>) -> String {
        title_with_year(title, year)
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
    async fn search(&self, query: &str, kind: Option<MediaKind>) -> Result<Vec<SearchResult>, MetadataError> {
        let query = query.to_lowercase();
        let series = self
            .series
            .lock()
            .unwrap()
            .values()
            .map(|m| result(MediaKind::Series, m.source, &m.title, m.year, &m.description.overview))
            .collect::<Vec<_>>();
        let movies = self
            .movies
            .lock()
            .unwrap()
            .values()
            .map(|m| result(MediaKind::Movie, m.source, &m.title, m.year, &m.description.overview))
            .collect::<Vec<_>>();
        let mut results: Vec<_> = series
            .into_iter()
            .chain(movies)
            .filter(|r| kind.is_none_or(|kind| r.kind == kind) && r.title.to_lowercase().contains(&query))
            .collect();
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

fn result(kind: MediaKind, source: ExternalId, title: &str, year: Option<i16>, overview: &str) -> SearchResult {
    let (title, original_title, overview) = (title.into(), title.into(), overview.into());
    SearchResult { kind, source, title, original_title, year, poster_path: None, overview }
}

pub struct App {
    _dir: TempDir,
    pub db: Database,
    pub clock: Arc<TestClock>,
    pub provider: Arc<StaticMetadata>,
    pub library: Library,
    pub calendar: Calendar,
    pub metadata: MetadataService,
}

impl App {
    pub async fn new() -> Self {
        let db = Database::open_in_memory().await.unwrap();
        let clock = Arc::new(TestClock::at(TODAY.at(12, 0, 0, 0).in_tz("Europe/Berlin").unwrap()));
        let provider = Arc::new(StaticMetadata::default());
        let repo = Arc::new(db.clone());
        let dir = TempDir::new().unwrap();
        let events = publisher(&db);
        let library = Library::new(repo.clone(), repo.clone(), clock.clone(), events.clone());
        let calendar = Calendar::new(repo.clone(), repo.clone(), clock.clone());
        let metadata =
            MetadataService::new(repo.clone(), repo, provider.clone(), Arc::new(TitleFolders), clock.clone(), events);
        Self { _dir: dir, db, clock, provider, library, calendar, metadata }
    }

    pub async fn events(&self) -> Vec<Event> {
        let recorded = EventLog::new(self.db.clone()).read_after(None, 100).await.unwrap();
        recorded.into_iter().map(|recorded| recorded.event).collect()
    }
}
