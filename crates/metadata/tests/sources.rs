use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{ExternalId, Live, MediaKind, MovieMetadata, Releases, SeriesMetadata, SourceStatus};
use yokoku_library::ports::{MetadataError, MetadataProvider, SearchResult};
use yokoku_metadata::Sources;

/// Answers every lookup with the id it was asked for; searches find one series and one movie.
struct Stub(fn(u64) -> ExternalId);

fn result(kind: MediaKind, source: ExternalId) -> SearchResult {
    SearchResult { kind, source, title: String::new(), original_title: String::new(), year: None, poster_path: None }
}

#[async_trait]
impl MetadataProvider for Stub {
    async fn search(&self, _query: &str) -> Result<Vec<SearchResult>, MetadataError> {
        Ok(vec![result(MediaKind::Series, self.0(1)), result(MediaKind::Movie, self.0(2))])
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        Ok(SeriesMetadata {
            source,
            title: String::new(),
            original_title: String::new(),
            alternate_titles: Vec::new(),
            year: None,
            poster_path: None,
            status: SourceStatus::Unknown,
            seasons: Vec::new(),
        })
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        Ok(MovieMetadata {
            source,
            title: String::new(),
            original_title: String::new(),
            alternate_titles: Vec::new(),
            year: None,
            poster_path: None,
            releases: Releases::default(),
        })
    }
}

fn sources(tvdb: bool) -> Sources {
    Sources::new(Arc::new(Stub(ExternalId::Tmdb)), Arc::new(Stub(ExternalId::Tvdb)), Live::fixed(tvdb))
}

#[tokio::test]
async fn searches_take_series_from_tvdb_once_it_is_set() {
    let sources: Vec<_> = sources(true).search("x").await.unwrap().into_iter().map(|r| r.source).collect();

    assert_eq!(sources, [ExternalId::Tmdb(2), ExternalId::Tvdb(1)]);
}

#[tokio::test]
async fn searches_take_everything_from_tmdb_without_tvdb() {
    let sources: Vec<_> = sources(false).search("x").await.unwrap().into_iter().map(|r| r.source).collect();

    assert_eq!(sources, [ExternalId::Tmdb(1), ExternalId::Tmdb(2)]);
}

#[tokio::test]
async fn series_are_looked_up_where_their_id_belongs() {
    let sources = sources(true);

    assert_eq!(sources.series(ExternalId::Tmdb(7)).await.unwrap().source, ExternalId::Tmdb(7));
    assert_eq!(sources.series(ExternalId::Tvdb(7)).await.unwrap().source, ExternalId::Tvdb(7));
}

#[tokio::test]
async fn tvdb_series_are_unavailable_without_tvdb() {
    let error = sources(false).series(ExternalId::Tvdb(7)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Unavailable(_)));
}
