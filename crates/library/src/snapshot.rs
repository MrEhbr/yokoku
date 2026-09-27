use yokoku_domain::{Movie, Series};

use crate::{
    LibraryError,
    ports::{MovieRepo, SeriesRepo},
};

/// Every series and movie, fully loaded.
pub(crate) struct Snapshot {
    pub series: Vec<Series>,
    pub movies: Vec<Movie>,
}

impl Snapshot {
    pub async fn load(series_repo: &dyn SeriesRepo, movie_repo: &dyn MovieRepo) -> Result<Self, LibraryError> {
        Ok(Self { series: series_repo.all().await?, movies: movie_repo.all().await? })
    }
}
