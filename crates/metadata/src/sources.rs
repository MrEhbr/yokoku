use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{ExternalId, Live, MediaKind, MovieMetadata, SeriesMetadata};
use yokoku_library::ports::{MetadataError, MetadataProvider, SearchResult};

/// Looks each item up at the source its id names. While `use_tvdb`, searches find series only there.
pub struct Sources {
    tmdb: Arc<dyn MetadataProvider>,
    tvdb: Arc<dyn MetadataProvider>,
    use_tvdb: Live<bool>,
}

impl Sources {
    pub fn new(tmdb: Arc<dyn MetadataProvider>, tvdb: Arc<dyn MetadataProvider>, use_tvdb: Live<bool>) -> Self {
        Self { tmdb, tvdb, use_tvdb }
    }

    fn tvdb(&self, source: ExternalId) -> Result<&dyn MetadataProvider, MetadataError> {
        self.use_tvdb
            .current()
            .then_some(self.tvdb.as_ref())
            .ok_or_else(|| MetadataError::Unavailable(format!("no TVDB API key configured to look up {source}").into()))
    }
}

#[async_trait]
impl MetadataProvider for Sources {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MetadataError> {
        if !self.use_tvdb.current() {
            return self.tmdb.search(query).await;
        }
        let mut results = self.tmdb.search(query).await?;
        results.retain(|result| result.kind == MediaKind::Movie);
        results.extend(self.tvdb.search(query).await?.into_iter().filter(|result| result.kind == MediaKind::Series));
        Ok(results)
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        match source {
            ExternalId::Tmdb(_) => self.tmdb.series(source).await,
            ExternalId::Tvdb(_) => self.tvdb(source)?.series(source).await,
        }
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        self.tmdb.movie(source).await
    }
}
