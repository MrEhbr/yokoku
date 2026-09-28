use std::path::Path;

use yokoku_domain::{ArtworkKind, ItemId, MovieId, SeriesId};
use yokoku_library::ports::ArtworkCache;
use yokoku_system::ArtworkFiles;

/// Every file under `dir`, relative to it.
fn files(dir: &Path) -> Vec<String> {
    let mut files: Vec<_> = walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned())
        .collect();
    files.sort();
    files
}

#[tokio::test]
async fn a_stored_image_is_read_back_by_its_kind_and_name() {
    let dir = tempfile::tempdir().unwrap();
    let artwork = ArtworkFiles::new(dir.path().join("artwork"));
    let item = ItemId::Series(SeriesId::generate());

    assert_eq!(artwork.get(item, ArtworkKind::Poster, "a.jpg").await.unwrap(), None);
    artwork.put(item, ArtworkKind::Poster, "a.jpg", b"poster").await.unwrap();

    assert_eq!(artwork.get(item, ArtworkKind::Poster, "a.jpg").await.unwrap().as_deref(), Some(&b"poster"[..]));
    assert_eq!(artwork.get(item, ArtworkKind::Backdrop, "a.jpg").await.unwrap(), None);
    assert_eq!(artwork.get(item, ArtworkKind::Poster, "b.jpg").await.unwrap(), None);
}

#[tokio::test]
async fn a_new_image_replaces_the_old_one_of_its_kind_only() {
    let dir = tempfile::tempdir().unwrap();
    let artwork = ArtworkFiles::new(dir.path());
    let (series, movie) = (ItemId::Series(SeriesId::generate()), ItemId::Movie(MovieId::generate()));
    artwork.put(series, ArtworkKind::Poster, "old.jpg", b"old").await.unwrap();
    artwork.put(series, ArtworkKind::Logo, "logo.png", b"logo").await.unwrap();
    artwork.put(movie, ArtworkKind::Poster, "old.jpg", b"movie").await.unwrap();

    artwork.put(series, ArtworkKind::Poster, "new.jpg", b"new").await.unwrap();

    let ItemId::Series(series) = series else { unreachable!() };
    let ItemId::Movie(movie) = movie else { unreachable!() };
    assert_eq!(
        files(dir.path()),
        [
            format!("movie/{movie}/poster-old.jpg"),
            format!("series/{series}/logo-logo.png"),
            format!("series/{series}/poster-new.jpg"),
        ]
    );
}

#[tokio::test]
async fn removing_an_item_removes_all_its_images_only() {
    let dir = tempfile::tempdir().unwrap();
    let artwork = ArtworkFiles::new(dir.path());
    let (series, movie) = (SeriesId::generate(), MovieId::generate());
    artwork.put(ItemId::Series(series), ArtworkKind::Poster, "a.jpg", b"a").await.unwrap();
    artwork.put(ItemId::Series(series), ArtworkKind::Backdrop, "b.jpg", b"b").await.unwrap();
    artwork.put(ItemId::Movie(movie), ArtworkKind::Poster, "c.jpg", b"c").await.unwrap();

    artwork.remove(ItemId::Series(series)).await.unwrap();
    artwork.remove(ItemId::Series(SeriesId::generate())).await.unwrap();

    assert_eq!(files(dir.path()), [format!("movie/{movie}/poster-c.jpg")]);
}

#[tokio::test]
async fn removing_before_any_image_is_stored_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let artwork = ArtworkFiles::new(dir.path().join("missing"));

    artwork.remove(ItemId::Movie(MovieId::generate())).await.unwrap();
}
