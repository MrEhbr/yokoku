use std::sync::Arc;

use yokoku_domain::{Clock, ExternalId, ItemId, MediaKind, MonitorPreset, Movie, MovieId, Series, SeriesId};
use yokoku_events::Event;

use crate::{
    LibraryError,
    ports::{MetadataProvider, MovieRepo, SearchResult, SeriesRepo},
};

/// Use cases that read from the metadata source.
pub struct MetadataSync {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    metadata: Arc<dyn MetadataProvider>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub result: SearchResult,
    pub in_library: bool,
}

#[derive(Debug, Default)]
pub struct RefreshReport {
    pub refreshed: usize,
    pub failures: Vec<RefreshFailure>,
}

#[derive(Debug)]
pub struct RefreshFailure {
    pub item: ItemId,
    pub error: LibraryError,
}

impl MetadataSync {
    pub fn new(
        series: Arc<dyn SeriesRepo>,
        movies: Arc<dyn MovieRepo>,
        metadata: Arc<dyn MetadataProvider>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { series, movies, metadata, clock }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchHit>, LibraryError> {
        let mut hits = Vec::new();
        for result in self.metadata.search(query).await? {
            let in_library = match result.kind {
                MediaKind::Series => self.series.find_by_source(result.source).await?.is_some(),
                MediaKind::Movie => self.movies.find_by_source(result.source).await?.is_some(),
            };
            hits.push(SearchHit { result, in_library });
        }
        Ok(hits)
    }

    pub async fn add_series(&self, source: ExternalId, preset: MonitorPreset) -> Result<Series, LibraryError> {
        if self.series.find_by_source(source).await?.is_some() {
            return Err(LibraryError::AlreadyInLibrary(source));
        }
        let metadata = self.metadata.series(source).await?;
        let now = self.clock.now();

        let series = Series::add(metadata, preset, now.date(), now.timestamp());
        let added = Event::SeriesAdded { series: series.id, title: series.title.clone() };
        self.series.save(&series, &[added]).await?;
        Ok(series)
    }

    pub async fn add_movie(&self, source: ExternalId, monitored: bool) -> Result<Movie, LibraryError> {
        if self.movies.find_by_source(source).await?.is_some() {
            return Err(LibraryError::AlreadyInLibrary(source));
        }
        let metadata = self.metadata.movie(source).await?;

        let movie = Movie::add(metadata, monitored, self.clock.now().timestamp());
        let added = Event::MovieAdded { movie: movie.id, title: movie.title.clone() };
        self.movies.save(&movie, &[added]).await?;
        Ok(movie)
    }

    pub async fn refresh_series(&self, id: SeriesId) -> Result<Series, LibraryError> {
        let mut series = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?;
        let metadata = self.metadata.series(series.source).await?;

        series.refresh(metadata, self.clock.now().timestamp());
        self.series.save(&series, &[]).await?;
        Ok(series)
    }

    pub async fn refresh_movie(&self, id: MovieId) -> Result<Movie, LibraryError> {
        let mut movie = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?;
        let metadata = self.metadata.movie(movie.source).await?;

        movie.refresh(metadata, self.clock.now().timestamp());
        self.movies.save(&movie, &[]).await?;
        Ok(movie)
    }

    /// Refreshes every item; one item's failure does not stop the others.
    pub async fn refresh_all(&self) -> Result<RefreshReport, LibraryError> {
        let mut report = RefreshReport::default();
        for id in self.series.ids().await? {
            report.record(ItemId::Series(id), self.refresh_series(id).await.map(drop));
        }
        for id in self.movies.ids().await? {
            report.record(ItemId::Movie(id), self.refresh_movie(id).await.map(drop));
        }
        Ok(report)
    }
}

impl RefreshReport {
    fn record(&mut self, item: ItemId, outcome: Result<(), LibraryError>) {
        match outcome {
            Ok(()) => self.refreshed += 1,
            Err(error) => self.failures.push(RefreshFailure { item, error }),
        }
    }
}
