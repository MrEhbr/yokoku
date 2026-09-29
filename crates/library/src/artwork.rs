use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{Artwork, ArtworkKind, ExternalId, ItemId};
use yokoku_events::{Handler, HandlerError, MovieRemoved, SeriesRemoved};

use crate::{
    LibraryError,
    ports::{ArtworkCache, ArtworkSource, MovieRepo, SeriesRepo},
};

/// An artwork image with its media type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

/// Item artwork, fetched from the metadata source once and then served from the cache.
pub struct Artworks {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    source: Arc<dyn ArtworkSource>,
    cache: Arc<dyn ArtworkCache>,
}

impl Artworks {
    pub fn new(
        series: Arc<dyn SeriesRepo>,
        movies: Arc<dyn MovieRepo>,
        source: Arc<dyn ArtworkSource>,
        cache: Arc<dyn ArtworkCache>,
    ) -> Self {
        Self { series, movies, source, cache }
    }

    /// The item's `kind` image; `None` when its source has none.
    pub async fn image(&self, item: ItemId, kind: ArtworkKind) -> Result<Option<Image>, LibraryError> {
        let (source, artwork) = self.artwork(item).await?;
        let Some(path) = artwork.get(kind) else { return Ok(None) };
        let Some(name) = artwork_name(path) else { return Ok(None) };
        let content_type = content_type(name);
        if let Some(bytes) = self.cache.get(item, kind, name).await? {
            return Ok(Some(Image { bytes, content_type }));
        }
        let bytes = self.source.fetch(source, kind, path).await?;
        self.cache.put(item, kind, name, &bytes).await?;
        Ok(Some(Image { bytes, content_type }))
    }

    /// A small version of the image at `path` of `source`, for an item not in the library;
    /// fetched on every call. `None` when the path does not end in a plain file name.
    pub async fn preview(
        &self,
        source: ExternalId,
        kind: ArtworkKind,
        path: &str,
    ) -> Result<Option<Image>, LibraryError> {
        let Some(name) = artwork_name(path) else { return Ok(None) };
        let bytes = self.source.fetch_thumbnail(source, kind, path).await?;
        Ok(Some(Image { bytes, content_type: content_type(name) }))
    }

    async fn artwork(&self, item: ItemId) -> Result<(ExternalId, Artwork), LibraryError> {
        Ok(match item {
            ItemId::Series(id) => {
                let series = self.series.get(id).await?.ok_or(LibraryError::SeriesNotFound(id))?;
                (series.source, series.artwork)
            },
            ItemId::Movie(id) => {
                let movie = self.movies.get(id).await?.ok_or(LibraryError::MovieNotFound(id))?;
                (movie.source, movie.artwork)
            },
        })
    }
}

/// The last segment of an artwork path, e.g. `81189-10.jpg`; `None` when it is not a plain file name.
pub fn artwork_name(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    let plain = name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    (plain && !name.is_empty() && !name.starts_with('.')).then_some(name)
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    }
}

#[async_trait]
impl Handler<SeriesRemoved> for Artworks {
    async fn handle(&self, event: &SeriesRemoved) -> Result<(), HandlerError> {
        Ok(self.cache.remove(ItemId::Series(event.series)).await?)
    }
}

#[async_trait]
impl Handler<MovieRemoved> for Artworks {
    async fn handle(&self, event: &MovieRemoved) -> Result<(), HandlerError> {
        Ok(self.cache.remove(ItemId::Movie(event.movie)).await?)
    }
}
