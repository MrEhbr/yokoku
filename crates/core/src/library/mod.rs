//! Catalog, monitoring, metadata refresh and calendar queries.

mod artwork;
mod calendar;
mod error;
mod files;
mod listing;
mod metadata;
pub mod ports;
mod retry;
mod snapshot;

use std::sync::Arc;

pub use artwork::{Artworks, Image, artwork_name};
pub use calendar::{
    Calendar, CalendarEntry, CalendarRelease, Missing, MissingEpisode, MissingMovie, MissingSeries, month_of, week_of,
};
pub use error::LibraryError;
pub use files::FileTracker;
use jiff::civil::Date;
pub use listing::{LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus};
pub use metadata::{MetadataService, RefreshFailure, RefreshReport, SearchHit};
use tracing::{info, instrument};
use yokoku_domain::{
    Clock, EpisodeRef, ExternalId, Movie, MovieId, Numbering, Series, SeriesId,
    events::{MovieRemoved, SeriesRemoved},
};

use self::{
    ports::{MovieRepo, SeriesRepo},
    snapshot::Snapshot,
};
use crate::events::Publisher;

/// Queries and changes that need no metadata source.
pub struct Library {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    clock: Arc<dyn Clock>,
    events: Publisher,
}

impl Library {
    pub fn new(
        series: Arc<dyn SeriesRepo>,
        movies: Arc<dyn MovieRepo>,
        clock: Arc<dyn Clock>,
        events: Publisher,
    ) -> Self {
        Self { series, movies, clock, events }
    }

    pub fn today(&self) -> Date {
        self.clock.now().date()
    }

    pub async fn list(&self, filter: LibraryFilter, sort: LibrarySort) -> Result<Vec<LibraryEntry>, LibraryError> {
        let today = self.today();
        let snapshot = Snapshot::load(self.series.as_ref(), self.movies.as_ref()).await?;
        let series = snapshot.series.iter().map(|series| LibraryEntry::from_series(series, today));
        let movies = snapshot.movies.iter().map(|movie| LibraryEntry::from_movie(movie, today));
        let mut entries: Vec<_> = series.chain(movies).collect();

        entries.retain(|entry| filter.matches(entry));
        entries.sort_by(|a, b| sort.compare(a, b));
        Ok(entries)
    }

    pub async fn series(&self, id: SeriesId) -> Result<Series, LibraryError> {
        self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))
    }

    pub async fn movie(&self, id: MovieId) -> Result<Movie, LibraryError> {
        self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))
    }

    pub async fn find_series(&self, source: ExternalId) -> Result<Option<Series>, LibraryError> {
        Ok(self.series.find_by_source(source).await?)
    }

    pub async fn find_movie(&self, source: ExternalId) -> Result<Option<Movie>, LibraryError> {
        Ok(self.movies.find_by_source(source).await?)
    }

    pub async fn set_series_monitored(&self, id: SeriesId, monitored: bool) -> Result<(), LibraryError> {
        self.update_series(id, |series| {
            series.monitored = monitored;
            Ok(())
        })
        .await
    }

    pub async fn set_season_monitored(&self, id: SeriesId, season: u16, monitored: bool) -> Result<(), LibraryError> {
        self.update_series(id, |series| {
            series.season_mut(season).ok_or(LibraryError::SeasonNotFound(season))?.monitored = monitored;
            Ok(())
        })
        .await
    }

    pub async fn set_episode_monitored(
        &self,
        id: SeriesId,
        episode: EpisodeRef,
        monitored: bool,
    ) -> Result<(), LibraryError> {
        self.update_series(id, |series| {
            series.episode_mut(episode).ok_or(LibraryError::EpisodeNotFound(episode))?.monitored = monitored;
            Ok(())
        })
        .await
    }

    pub async fn set_numbering(&self, id: SeriesId, numbering: Numbering) -> Result<(), LibraryError> {
        self.update_series(id, |series| {
            series.numbering = numbering;
            Ok(())
        })
        .await
    }

    pub async fn set_movie_monitored(&self, id: MovieId, monitored: bool) -> Result<(), LibraryError> {
        retry::on_conflict(|| async move {
            let mut movie = self.movie(id).await?;
            movie.monitored = monitored;
            Ok(self.movies.save(&mut movie).await?)
        })
        .await
    }

    #[instrument(skip_all, fields(series = %id, delete_files))]
    pub async fn remove_series(&self, id: SeriesId, delete_files: bool) -> Result<(), LibraryError> {
        let series = self.series(id).await?;
        self.series.remove(id).await?;
        info!(title = %series.title, "series removed");
        self.events.publish(SeriesRemoved { series: id, title: series.title, delete_files }).await;
        Ok(())
    }

    #[instrument(skip_all, fields(movie = %id, delete_files))]
    pub async fn remove_movie(&self, id: MovieId, delete_files: bool) -> Result<(), LibraryError> {
        let movie = self.movie(id).await?;
        self.movies.remove(id).await?;
        info!(title = %movie.title, "movie removed");
        self.events.publish(MovieRemoved { movie: id, title: movie.title, delete_files }).await;
        Ok(())
    }

    async fn update_series(
        &self,
        id: SeriesId,
        change: impl Fn(&mut Series) -> Result<(), LibraryError>,
    ) -> Result<(), LibraryError> {
        let change = &change;
        retry::on_conflict(|| async move {
            let mut series = self.series(id).await?;
            change(&mut series)?;
            Ok(self.series.save(&mut series).await?)
        })
        .await
    }
}
