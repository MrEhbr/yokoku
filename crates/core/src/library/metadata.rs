use std::{path::PathBuf, sync::Arc};

use tracing::{debug, info, instrument};
use yokoku_domain::{
    Clock, ExternalId, ItemFolder, ItemId, MediaKind, MonitorPreset, Movie, MovieId, Series, SeriesId,
};

use crate::{
    events::{EpisodesRenumbered, MovieAdded, Publisher, SeriesAdded},
    library::{
        LibraryError,
        ports::{FolderNames, MetadataProvider, MovieRepo, SearchResult, SeriesRepo},
        retry,
    },
};

/// Use cases that read from the metadata source.
pub struct MetadataService {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    metadata: Arc<dyn MetadataProvider>,
    folders: Arc<dyn FolderNames>,
    clock: Arc<dyn Clock>,
    events: Publisher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub result: SearchResult,
    /// The item holding this result when it is already in the library.
    pub in_library: Option<ItemId>,
    /// The folder name `add_series` or `add_movie` gives the item when the caller gives none.
    pub folder: String,
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

impl MetadataService {
    pub fn new(
        series: Arc<dyn SeriesRepo>,
        movies: Arc<dyn MovieRepo>,
        metadata: Arc<dyn MetadataProvider>,
        folders: Arc<dyn FolderNames>,
        clock: Arc<dyn Clock>,
        events: Publisher,
    ) -> Self {
        Self { series, movies, metadata, folders, clock, events }
    }

    /// Movies and series matching `query`, or only those of `kind`.
    pub async fn search(&self, query: &str, kind: Option<MediaKind>) -> Result<Vec<SearchHit>, LibraryError> {
        let mut hits = Vec::new();
        for result in self.metadata.search(query, kind).await? {
            let (in_library, folder) = match result.kind {
                MediaKind::Series => (
                    self.series.find_by_source(result.source).await?.map(|series| ItemId::Series(series.id)),
                    self.folders.series_folder(&result.title, result.year),
                ),
                MediaKind::Movie => (
                    self.movies.find_by_source(result.source).await?.map(|movie| ItemId::Movie(movie.id)),
                    self.folders.movie_folder(&result.title, result.year),
                ),
            };
            hits.push(SearchHit { result, in_library, folder });
        }
        Ok(hits)
    }

    /// Adds the series in `root`, in the folder `folder` or else the one its naming pattern gives.
    /// The caller checks that `root` is a series root folder.
    #[instrument(skip_all, fields(%source))]
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
        let folder = ItemFolder::new(
            root,
            folder.unwrap_or_else(|| self.folders.series_folder(&metadata.title, metadata.year)),
        )?;
        if self.series.find_by_folder(&folder).await?.is_some() {
            return Err(LibraryError::FolderTaken(folder.path()));
        }
        let now = self.clock.now();

        let mut series = Series::add(metadata, folder, preset, now.date(), now.timestamp());
        self.series.save(&mut series).await?;
        info!(series = %series.id, title = %series.title, "series added");
        self.events.publish(SeriesAdded { series: series.id, title: series.title.clone() }).await;
        Ok(series)
    }

    /// Adds the movie in `root`, in the folder `folder` or else the one its naming pattern gives.
    /// The caller checks that `root` is a movie root folder.
    #[instrument(skip_all, fields(%source))]
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
        let folder =
            ItemFolder::new(root, folder.unwrap_or_else(|| self.folders.movie_folder(&metadata.title, metadata.year)))?;
        if self.movies.find_by_folder(&folder).await?.is_some() {
            return Err(LibraryError::FolderTaken(folder.path()));
        }

        let mut movie = Movie::add(metadata, folder, monitored, self.clock.now().timestamp());
        self.movies.save(&mut movie).await?;
        info!(movie = %movie.id, title = %movie.title, "movie added");
        self.events.publish(MovieAdded { movie: movie.id, title: movie.title.clone() }).await;
        Ok(movie)
    }

    #[instrument(skip_all, fields(series = %id))]
    pub async fn refresh_series(&self, id: SeriesId) -> Result<Series, LibraryError> {
        let source = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?.source;
        let metadata = self.metadata.series(source).await?;

        let metadata = &metadata;
        let (series, files) = retry::on_conflict(|| async move {
            let mut series = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?;
            let files = series.refresh(metadata.clone(), self.clock.now().timestamp());
            self.series.save(&mut series).await?;
            debug!("series refreshed");
            Ok((series, files))
        })
        .await?;
        if !files.is_empty() {
            info!(files = files.len(), "episodes holding files were renumbered");
            self.events.publish(EpisodesRenumbered { series: id, files }).await;
        }
        Ok(series)
    }

    #[instrument(skip_all, fields(movie = %id))]
    pub async fn refresh_movie(&self, id: MovieId) -> Result<Movie, LibraryError> {
        let source = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?.source;
        let metadata = self.metadata.movie(source).await?;

        let metadata = &metadata;
        retry::on_conflict(|| async move {
            let mut movie = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?;
            movie.refresh(metadata.clone(), self.clock.now().timestamp());
            self.movies.save(&mut movie).await?;
            debug!("movie refreshed");
            Ok(movie)
        })
        .await
    }

    /// Refreshes every item; one item's failure does not stop the others.
    #[instrument(skip_all)]
    pub async fn refresh_all(&self) -> Result<RefreshReport, LibraryError> {
        self.refresh_many(false).await
    }

    /// Refreshes the items whose `needs_refresh` holds; one item's failure does not stop the others.
    #[instrument(skip_all)]
    pub async fn refresh_due(&self) -> Result<RefreshReport, LibraryError> {
        self.refresh_many(true).await
    }

    async fn refresh_many(&self, due_only: bool) -> Result<RefreshReport, LibraryError> {
        let now = self.clock.now();
        let (timestamp, today) = (now.timestamp(), now.date());
        let mut report = RefreshReport::default();
        for id in self.series.ids().await? {
            if due_only && !self.series.get(id).await?.is_some_and(|series| series.needs_refresh(timestamp, today)) {
                continue;
            }
            report.record(ItemId::Series(id), self.refresh_series(id).await.map(drop));
        }
        for id in self.movies.ids().await? {
            if due_only && !self.movies.get(id).await?.is_some_and(|movie| movie.needs_refresh(timestamp, today)) {
                continue;
            }
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
