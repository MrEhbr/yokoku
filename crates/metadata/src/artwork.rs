use async_trait::async_trait;
use yokoku_domain::{ArtworkKind, ExternalId};
use yokoku_library::ports::{ArtworkSource, MetadataError};

use crate::http::{Http, invalid, unavailable};

/// TMDB image paths are relative to this, followed by a size.
const TMDB_IMAGES: &str = "https://image.tmdb.org/t/p";
/// TVDB image paths are URLs under this.
const TVDB_IMAGES: &str = "https://artworks.thetvdb.com/";

/// Downloads artwork from TMDB's image server and TheTVDB's artwork server, and nowhere else.
pub struct ArtworkFetcher {
    http: Http,
    tmdb: String,
    tvdb: String,
}

impl ArtworkFetcher {
    pub fn new() -> Self {
        Self::with_servers(TMDB_IMAGES, TVDB_IMAGES)
    }

    fn with_servers(tmdb: &str, tvdb: &str) -> Self {
        Self { http: Http::new("artwork server"), tmdb: tmdb.to_owned(), tvdb: tvdb.to_owned() }
    }

    /// `size` is a TMDB size; TVDB images come in one size.
    fn url(&self, source: ExternalId, size: &str, path: &str) -> Result<String, MetadataError> {
        match source {
            ExternalId::Tmdb(_) if path.starts_with('/') => Ok(format!("{}/{size}{path}", self.tmdb)),
            ExternalId::Tvdb(_) if path.starts_with(&self.tvdb) => Ok(path.to_owned()),
            _ => Err(invalid(PathOutsideSource(format!("{source} has the image path {path:?}")))),
        }
    }

    async fn get(&self, source: ExternalId, url: String) -> Result<Vec<u8>, MetadataError> {
        let response = self.http.send(self.http.get(url), Some(source)).await?;
        Ok(response.bytes().await.map_err(unavailable)?.to_vec())
    }
}

impl Default for ArtworkFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Widths close to what pages show: posters in cards, backdrops across the page, logos over them.
fn tmdb_size(kind: ArtworkKind) -> &'static str {
    match kind {
        ArtworkKind::Poster => "w342",
        ArtworkKind::Backdrop => "w1280",
        ArtworkKind::Logo => "w500",
    }
}

/// Widths for small previews, like a search result's poster.
fn tmdb_thumbnail_size(kind: ArtworkKind) -> &'static str {
    match kind {
        ArtworkKind::Poster => "w342",
        ArtworkKind::Logo => "w154",
        ArtworkKind::Backdrop => "w300",
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}, which is not at its source's image server")]
struct PathOutsideSource(String);

#[async_trait]
impl ArtworkSource for ArtworkFetcher {
    async fn fetch(&self, source: ExternalId, kind: ArtworkKind, path: &str) -> Result<Vec<u8>, MetadataError> {
        self.get(source, self.url(source, tmdb_size(kind), path)?).await
    }

    async fn fetch_thumbnail(
        &self,
        source: ExternalId,
        kind: ArtworkKind,
        path: &str,
    ) -> Result<Vec<u8>, MetadataError> {
        self.get(source, self.url(source, tmdb_thumbnail_size(kind), path)?).await
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};
    use yokoku_domain::{ArtworkKind, ExternalId};
    use yokoku_library::ports::{ArtworkSource, MetadataError};

    use super::ArtworkFetcher;

    #[rstest]
    #[case::poster(ArtworkKind::Poster, "/w342/abc.jpg")]
    #[case::backdrop(ArtworkKind::Backdrop, "/w1280/abc.jpg")]
    #[case::logo(ArtworkKind::Logo, "/w500/abc.jpg")]
    #[tokio::test]
    async fn tmdb_artwork_comes_from_the_image_server_at_its_kinds_size(
        #[case] kind: ArtworkKind,
        #[case] served_at: &str,
    ) {
        let server = MockServer::start().await;
        let image = ResponseTemplate::new(200).set_body_bytes(b"image".to_vec());
        Mock::given(path(served_at)).respond_with(image).mount(&server).await;
        let fetcher = ArtworkFetcher::with_servers(&server.uri(), "https://artworks.thetvdb.com/");

        assert_eq!(fetcher.fetch(ExternalId::Tmdb(1), kind, "/abc.jpg").await.unwrap(), b"image");
    }

    #[rstest]
    #[case::poster(ArtworkKind::Poster, "/w342/abc.jpg")]
    #[case::backdrop(ArtworkKind::Backdrop, "/w300/abc.jpg")]
    #[case::logo(ArtworkKind::Logo, "/w154/abc.jpg")]
    #[tokio::test]
    async fn tmdb_thumbnails_come_at_a_small_size(#[case] kind: ArtworkKind, #[case] served_at: &str) {
        let server = MockServer::start().await;
        let image = ResponseTemplate::new(200).set_body_bytes(b"thumbnail".to_vec());
        Mock::given(path(served_at)).respond_with(image).mount(&server).await;
        let fetcher = ArtworkFetcher::with_servers(&server.uri(), "https://artworks.thetvdb.com/");

        let thumbnail = fetcher.fetch_thumbnail(ExternalId::Tmdb(1), kind, "/abc.jpg").await.unwrap();

        assert_eq!(thumbnail, b"thumbnail");
    }

    #[tokio::test]
    async fn tvdb_artwork_comes_from_its_stored_url() {
        let server = MockServer::start().await;
        let image = ResponseTemplate::new(200).set_body_bytes(b"image".to_vec());
        Mock::given(path("/banners/fanart/81189-21.jpg")).respond_with(image).mount(&server).await;
        let fetcher = ArtworkFetcher::with_servers("https://image.tmdb.org/t/p", &format!("{}/", server.uri()));
        let url = format!("{}/banners/fanart/81189-21.jpg", server.uri());

        assert_eq!(fetcher.fetch(ExternalId::Tvdb(81189), ArtworkKind::Backdrop, &url).await.unwrap(), b"image");
    }

    #[tokio::test]
    async fn paths_off_the_sources_image_servers_are_refused() {
        let fetcher = ArtworkFetcher::new();

        for (source, path) in [
            (ExternalId::Tvdb(1), "http://artworks.thetvdb.com/a.jpg"),
            (ExternalId::Tvdb(1), "https://example.com/a.jpg"),
            (ExternalId::Tvdb(1), "/a.jpg"),
            (ExternalId::Tmdb(1), "https://image.tmdb.org/t/p/w342/a.jpg"),
        ] {
            let error = fetcher.fetch(source, ArtworkKind::Poster, path).await.unwrap_err();
            assert!(matches!(error, MetadataError::Invalid(_)), "{path}: {error:?}");
        }
    }

    #[tokio::test]
    async fn missing_artwork_is_not_found() {
        let server = MockServer::start().await;
        let fetcher = ArtworkFetcher::with_servers(&server.uri(), "https://artworks.thetvdb.com/");

        let error = fetcher.fetch(ExternalId::Tmdb(7), ArtworkKind::Logo, "/gone.png").await.unwrap_err();

        assert!(matches!(error, MetadataError::NotFound(ExternalId::Tmdb(7))));
    }
}
