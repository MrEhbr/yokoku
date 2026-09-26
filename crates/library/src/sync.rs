use std::{path::PathBuf, sync::Arc};

use yokoku_domain::{
    Clock, ExternalId, ItemFolder, ItemId, MediaKind, MonitorPreset, Movie, MovieId, Series, SeriesId,
};
use yokoku_events::Event;

use crate::{
    LibraryError,
    ports::{FolderNames, MetadataProvider, MovieRepo, SearchResult, SeriesRepo},
    retry,
};

/// Use cases that read from the metadata source.
pub struct MetadataSync {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    metadata: Arc<dyn MetadataProvider>,
    folders: Arc<dyn FolderNames>,
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
        folders: Arc<dyn FolderNames>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { series, movies, metadata, folders, clock }
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

    /// Adds the series in `root`, in the folder `folder` or else the one its naming pattern gives.
    /// The caller checks that `root` is a series root folder.
    pub async fn add_series(
        &self,
        source: ExternalId,
        preset: MonitorPreset,
        root: PathBuf,
        folder: Option<String>,
    ) -> Result<Series, LibraryError> {
        if self.series.find_by_source(source).await?.is_some() {
            return Err(LibraryError::AlreadyInLibrary(source));
        }
        let metadata = self.metadata.series(source).await?;
        let folder = ItemFolder::new(root, folder.unwrap_or_else(|| self.folders.series_folder(&metadata)))?;
        if self.series.find_by_folder(&folder).await?.is_some() {
            return Err(LibraryError::FolderTaken(folder.path()));
        }
        let now = self.clock.now();

        let mut series = Series::add(metadata, folder, preset, now.date(), now.timestamp());
        let added = Event::SeriesAdded { series: series.id, title: series.title.clone() };
        self.series.save(&mut series, &[added]).await?;
        Ok(series)
    }

    /// Adds the movie in `root`, in the folder `folder` or else the one its naming pattern gives.
    /// The caller checks that `root` is a movie root folder.
    pub async fn add_movie(
        &self,
        source: ExternalId,
        monitored: bool,
        root: PathBuf,
        folder: Option<String>,
    ) -> Result<Movie, LibraryError> {
        if self.movies.find_by_source(source).await?.is_some() {
            return Err(LibraryError::AlreadyInLibrary(source));
        }
        let metadata = self.metadata.movie(source).await?;
        let folder = ItemFolder::new(root, folder.unwrap_or_else(|| self.folders.movie_folder(&metadata)))?;
        if self.movies.find_by_folder(&folder).await?.is_some() {
            return Err(LibraryError::FolderTaken(folder.path()));
        }

        let mut movie = Movie::add(metadata, folder, monitored, self.clock.now().timestamp());
        let added = Event::MovieAdded { movie: movie.id, title: movie.title.clone() };
        self.movies.save(&mut movie, &[added]).await?;
        Ok(movie)
    }

    pub async fn refresh_series(&self, id: SeriesId) -> Result<Series, LibraryError> {
        let source = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?.source;
        let metadata = self.metadata.series(source).await?;

        let metadata = &metadata;
        retry::on_conflict(|| async move {
            let mut series = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?;
            series.refresh(metadata.clone(), self.clock.now().timestamp());
            self.series.save(&mut series, &[]).await?;
            Ok(series)
        })
        .await
    }

    pub async fn refresh_movie(&self, id: MovieId) -> Result<Movie, LibraryError> {
        let source = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?.source;
        let metadata = self.metadata.movie(source).await?;

        let metadata = &metadata;
        retry::on_conflict(|| async move {
            let mut movie = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?;
            movie.refresh(metadata.clone(), self.clock.now().timestamp());
            self.movies.save(&mut movie, &[]).await?;
            Ok(movie)
        })
        .await
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
