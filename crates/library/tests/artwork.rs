mod common;

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{App, ROOT, movie_metadata, series_metadata};
use rstest::rstest;
use tempfile::TempDir;
use yokoku_domain::{
    Artwork, ArtworkKind, ExternalId, ItemId, MonitorPreset, MovieMetadata, Releases, SeriesId, SeriesMetadata,
    SourceStatus,
};
use yokoku_events::{Handler, SeriesRemoved};
use yokoku_library::{
    Artworks, Image, LibraryError, artwork_name,
    ports::{ArtworkSource, MetadataError, SeriesRepo},
};
use yokoku_system::ArtworkFiles;

/// The metadata sources' image servers: every path has an image, and each fetch is recorded.
#[derive(Default)]
struct Servers(Mutex<Vec<(ArtworkKind, String)>>);

impl Servers {
    fn fetched(&self) -> Vec<(ArtworkKind, String)> {
        self.0.lock().unwrap().clone()
    }
}

#[async_trait]
impl ArtworkSource for Servers {
    async fn fetch(&self, _source: ExternalId, kind: ArtworkKind, path: &str) -> Result<Vec<u8>, MetadataError> {
        self.0.lock().unwrap().push((kind, path.to_owned()));
        Ok(format!("image of {path}").into_bytes())
    }

    async fn fetch_thumbnail(
        &self,
        _source: ExternalId,
        kind: ArtworkKind,
        path: &str,
    ) -> Result<Vec<u8>, MetadataError> {
        self.0.lock().unwrap().push((kind, format!("thumbnail {path}")));
        Ok(format!("thumbnail of {path}").into_bytes())
    }
}

struct Setup {
    app: App,
    servers: Arc<Servers>,
    artworks: Artworks,
    cache: TempDir,
}

async fn setup() -> Setup {
    let app = App::new().await;
    let servers = Arc::new(Servers::default());
    let cache = TempDir::new().unwrap();
    let repo = Arc::new(app.db.clone());
    let artworks = Artworks::new(repo.clone(), repo, servers.clone(), Arc::new(ArtworkFiles::new(cache.path())));
    Setup { app, servers, artworks, cache }
}

impl Setup {
    async fn add_series(&self, artwork: Artwork) -> SeriesId {
        let metadata = SeriesMetadata { artwork, ..series_metadata(1, "Frieren", SourceStatus::Returning, &[]) };
        self.app.provider.put_series(metadata);
        self.app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap().id
    }

    fn cached(&self) -> usize {
        files(self.cache.path())
    }
}

/// Files under `dir`, in any depth.
fn files(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries.map(|entry| entry.unwrap().path()).map(|path| if path.is_dir() { files(&path) } else { 1 }).sum()
    })
}

fn poster(path: &str) -> Artwork {
    Artwork { poster: Some(path.into()), ..Artwork::default() }
}

fn image(path: &str, content_type: &'static str) -> Option<Image> {
    Some(Image { bytes: format!("image of {path}").into_bytes(), content_type })
}

#[tokio::test]
async fn an_image_is_fetched_once_then_served_from_the_cache() {
    let setup = setup().await;
    let series = ItemId::Series(setup.add_series(poster("/frieren.jpg")).await);

    let first = setup.artworks.image(series, ArtworkKind::Poster).await.unwrap();
    let second = setup.artworks.image(series, ArtworkKind::Poster).await.unwrap();

    assert_eq!(first, image("/frieren.jpg", "image/jpeg"));
    assert_eq!(second, first);
    assert_eq!(setup.servers.fetched(), [(ArtworkKind::Poster, "/frieren.jpg".to_owned())]);
}

#[tokio::test]
async fn each_kind_is_its_own_image() {
    let setup = setup().await;
    let artwork = Artwork {
        poster: Some("/poster.jpg".into()),
        backdrop: Some("/backdrop.jpg".into()),
        logo: Some("/logo.png".into()),
    };
    let series = ItemId::Series(setup.add_series(artwork).await);

    let backdrop = setup.artworks.image(series, ArtworkKind::Backdrop).await.unwrap();
    let logo = setup.artworks.image(series, ArtworkKind::Logo).await.unwrap();

    assert_eq!(backdrop, image("/backdrop.jpg", "image/jpeg"));
    assert_eq!(logo, image("/logo.png", "image/png"));
    assert_eq!(
        setup.servers.fetched(),
        [(ArtworkKind::Backdrop, "/backdrop.jpg".to_owned()), (ArtworkKind::Logo, "/logo.png".to_owned()),]
    );
}

#[tokio::test]
async fn a_changed_path_fetches_the_new_image_and_drops_the_old() {
    let setup = setup().await;
    let id = setup.add_series(poster("/old.jpg")).await;
    setup.artworks.image(ItemId::Series(id), ArtworkKind::Poster).await.unwrap();
    let mut series = setup.app.library.series(id).await.unwrap();
    series.artwork.poster = Some("/new.jpg".into());
    SeriesRepo::save(&setup.app.db, &mut series).await.unwrap();

    let image_now = setup.artworks.image(ItemId::Series(id), ArtworkKind::Poster).await.unwrap();

    assert_eq!(image_now, image("/new.jpg", "image/jpeg"));
    assert_eq!(setup.cached(), 1);
}

#[tokio::test]
async fn a_kind_the_source_has_no_image_for_is_none() {
    let setup = setup().await;
    setup
        .app
        .provider
        .put_movie(MovieMetadata { artwork: poster("/dune.jpg"), ..movie_metadata(10, "Dune", Releases::default()) });
    let movie = setup.app.metadata.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();

    assert_eq!(setup.artworks.image(ItemId::Movie(movie.id), ArtworkKind::Logo).await.unwrap(), None);
    assert!(setup.servers.fetched().is_empty());
}

#[tokio::test]
async fn an_item_not_in_the_library_is_not_found() {
    let setup = setup().await;
    let missing = SeriesId::generate();

    let error = setup.artworks.image(ItemId::Series(missing), ArtworkKind::Poster).await.unwrap_err();

    assert!(matches!(error, LibraryError::SeriesNotFound(id) if id == missing));
}

#[tokio::test]
async fn a_preview_is_a_thumbnail_fetched_every_time_and_never_cached() {
    let setup = setup().await;

    let first = setup.artworks.preview(ExternalId::Tmdb(1), ArtworkKind::Poster, "/dune.png").await.unwrap();
    setup.artworks.preview(ExternalId::Tmdb(1), ArtworkKind::Poster, "/dune.png").await.unwrap();

    let thumbnail = Image { bytes: b"thumbnail of /dune.png".to_vec(), content_type: "image/png" };
    assert_eq!(first, Some(thumbnail));
    assert_eq!(setup.servers.fetched(), vec![(ArtworkKind::Poster, "thumbnail /dune.png".to_owned()); 2]);
    assert_eq!(setup.cached(), 0);
}

#[tokio::test]
async fn a_preview_path_without_a_file_name_is_none() {
    let setup = setup().await;

    let preview = setup.artworks.preview(ExternalId::Tmdb(1), ArtworkKind::Poster, "/").await.unwrap();

    assert_eq!(preview, None);
    assert!(setup.servers.fetched().is_empty());
}

#[tokio::test]
async fn a_removed_series_loses_its_cached_images() {
    let setup = setup().await;
    let artwork = Artwork { backdrop: Some("/backdrop.jpg".into()), ..poster("/poster.jpg") };
    let id = setup.add_series(artwork).await;
    setup.artworks.image(ItemId::Series(id), ArtworkKind::Poster).await.unwrap();
    setup.artworks.image(ItemId::Series(id), ArtworkKind::Backdrop).await.unwrap();

    setup.artworks.handle(&SeriesRemoved { series: id, title: "Frieren".into(), delete_files: false }).await.unwrap();

    assert_eq!(setup.cached(), 0);
}

#[rstest]
#[case::tmdb_path("/v1tRXZ4JtD2Iv6fjkPvT4GiwslV.jpg", Some("v1tRXZ4JtD2Iv6fjkPvT4GiwslV.jpg"))]
#[case::tvdb_url("https://artworks.thetvdb.com/banners/posters/81189-10.jpg", Some("81189-10.jpg"))]
#[case::no_file_name("https://artworks.thetvdb.com/banners/", None)]
#[case::hidden_file("/.jpg", None)]
#[case::other_characters("/a b.jpg", None)]
fn an_image_is_named_after_its_file(#[case] path: &str, #[case] name: Option<&str>) {
    assert_eq!(artwork_name(path), name);
}

#[tokio::test]
async fn a_webp_image_is_served_as_webp() {
    let setup = setup().await;
    let series = ItemId::Series(setup.add_series(poster("/frieren.WEBP")).await);

    let served = setup.artworks.image(series, ArtworkKind::Poster).await.unwrap();

    assert_eq!(served, image("/frieren.WEBP", "image/webp"));
}
