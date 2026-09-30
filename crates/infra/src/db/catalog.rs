use async_trait::async_trait;
use yokoku_core::media::ports::Catalog;
use yokoku_domain::{Movie, MovieId, Series, SeriesId, StorageError};

use crate::db::Database;

#[async_trait]
impl Catalog for Database {
    async fn all_series(&self) -> Result<Vec<Series>, StorageError> {
        Ok(self.load_all_series().await?)
    }

    async fn all_movies(&self) -> Result<Vec<Movie>, StorageError> {
        Ok(self.load_all_movies().await?)
    }

    async fn series(&self, id: SeriesId) -> Result<Option<Series>, StorageError> {
        Ok(self.load_series(id).await?)
    }

    async fn movie(&self, id: MovieId) -> Result<Option<Movie>, StorageError> {
        Ok(self.load_movie(id).await?)
    }
}
