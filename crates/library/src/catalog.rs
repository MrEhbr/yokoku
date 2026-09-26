use yokoku_domain::{Movie, Series};

use crate::{
    LibraryError,
    ports::{MovieRepo, SeriesRepo},
};

/// Every series and movie, fully loaded.
pub(crate) struct Catalog {
    pub series: Vec<Series>,
    pub movies: Vec<Movie>,
}

impl Catalog {
    /// Items removed while loading are skipped.
    pub async fn load(series_repo: &dyn SeriesRepo, movie_repo: &dyn MovieRepo) -> Result<Self, LibraryError> {
        let mut series = Vec::new();
        for id in series_repo.ids().await? {
            series.extend(series_repo.get(id).await?);
        }
        let mut movies = Vec::new();
        for id in movie_repo.ids().await? {
            movies.extend(movie_repo.get(id).await?);
        }
        Ok(Self { series, movies })
    }
}
