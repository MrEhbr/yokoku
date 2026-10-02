use std::{fs, os::unix::fs::PermissionsExt};

use common::{App, TODAY, frieren_metadata, now};
use yokoku_core::{
    events::Handler,
    library::ports::SeriesRepo,
    media::{MediaError, MediaFile},
};
use yokoku_domain::{
    ItemFolder, ItemId, MediaFileId, MonitorPreset, Series,
    events::{DeleteReason, Event, FileDeleted, MovieRemoved, SeriesAdded, SeriesRemoved},
};

use crate::common;

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv";
const E01_SUBTITLE: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.en.srt";
const E02: &str = "tv/Frieren (2023)/Season 02/Frieren (2023) - S02E01-E02.mkv";
const DUNE: &str = "movies/Dune (2021)/Dune (2021).mkv";

fn deleted_events(events: &[Event]) -> Vec<DeleteReason> {
    events.iter().filter_map(Event::get::<FileDeleted>).map(|deleted| deleted.reason).collect()
}

/// The id of the file at `relative` among `files`.
fn id_of(app: &App, files: &[MediaFile], relative: &str) -> MediaFileId {
    files.iter().find(|file| file.path == app.path(relative)).unwrap().id
}

#[tokio::test]
async fn deleting_removes_the_file_its_subtitles_and_empty_folders() {
    let app = App::new().await;
    let files = app.linked(&[E01, E01_SUBTITLE, E02]).await;

    let deleted =
        app.deleter().delete_files(ItemId::Series(app.frieren.id), &[id_of(&app, &files, E01)]).await.unwrap();

    assert_eq!(deleted.len(), 1);
    assert_eq!((app.path(E01).exists(), app.path(E01_SUBTITLE).exists()), (false, false));
    assert!(!app.path("tv/Frieren (2023)/Season 01").exists());
    assert!(app.path(E02).exists());
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::User]);
}

#[tokio::test]
async fn deleting_chosen_files_keeps_the_others() {
    let app = App::new().await;
    let files = app.linked(&[E01, E01_SUBTITLE, E02, DUNE]).await;
    let (e01, dune) = (id_of(&app, &files, E01), id_of(&app, &files, DUNE));

    let deleted = app.deleter().delete_files(ItemId::Series(app.frieren.id), &[e01, dune]).await.unwrap();

    assert_eq!(deleted.iter().map(|file| file.id).collect::<Vec<_>>(), [e01]);
    assert_eq!((app.path(E01).exists(), app.path(E01_SUBTITLE).exists()), (false, false));
    assert_eq!((app.path(E02).exists(), app.path(DUNE).exists()), (true, true));
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::User]);
}

#[tokio::test]
async fn deleting_no_file_of_the_item_fails() {
    let app = App::new().await;
    let files = app.linked(&[DUNE]).await;

    let error = app.deleter().delete_files(ItemId::Series(app.frieren.id), &[files[0].id]).await.unwrap_err();

    assert!(matches!(error, MediaError::NoFile), "{error}");
    assert!(app.path(DUNE).exists());
}

#[tokio::test]
async fn removing_a_series_with_its_files_deletes_them_all() {
    let app = App::new().await;
    app.linked(&[E01, E02, DUNE]).await;
    let removed = |delete_files| SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files };
    let deleter = app.deleter();

    deleter.handle(&removed(true)).await.unwrap();
    deleter.handle(&removed(true)).await.unwrap();

    assert_eq!((app.path(E01).exists(), app.path(E02).exists()), (false, false));
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::ItemRemoved; 2]);
}

#[tokio::test]
async fn removing_a_movie_with_its_file_deletes_only_it() {
    let app = App::new().await;
    app.linked(&[E01, DUNE]).await;

    app.deleter().handle(&MovieRemoved { movie: app.dune.id, title: "Dune".into(), delete_files: true }).await.unwrap();

    assert_eq!((app.path(DUNE).exists(), app.path(E01).exists()), (false, true));
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::ItemRemoved]);
}

#[tokio::test]
async fn files_kept_when_removing_a_series_return_when_it_is_added_again() {
    let app = App::new().await;
    app.linked(&[E01, E02, DUNE]).await;
    SeriesRepo::remove(&app.db, app.frieren.id).await.unwrap();

    app.deleter()
        .handle(&SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files: false })
        .await
        .unwrap();
    let folder = ItemFolder::new(app.path("tv"), "Frieren (2023)".into()).unwrap();
    let mut readded = Series::new(frieren_metadata(), folder, MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&app.db, &mut readded).await.unwrap();
    app.scanner.handle(&SeriesAdded { series: readded.id, title: "Frieren".into() }).await.unwrap();

    assert_eq!((app.path(E01).exists(), app.path(E02).exists()), (true, true));
    let series_files: Vec<_> =
        app.db_files().await.into_iter().filter(|file| file.target.item() != ItemId::Movie(app.dune.id)).collect();
    assert_eq!(series_files.len(), 2);
    assert!(series_files.iter().all(|file| file.target.item() == ItemId::Series(readded.id)));
    assert!(deleted_events(&app.events().await).is_empty());
}

#[tokio::test]
async fn a_folder_that_cannot_be_removed_does_not_keep_the_deleted_file() {
    let app = App::new().await;
    let files = app.linked(&[E01, E01_SUBTITLE]).await;
    let series = app.path("tv/Frieren (2023)");
    fs::set_permissions(&series, fs::Permissions::from_mode(0o555)).unwrap();

    let result = app.deleter().delete_files(ItemId::Series(app.frieren.id), &[id_of(&app, &files, E01)]).await;

    fs::set_permissions(&series, fs::Permissions::from_mode(0o755)).unwrap();
    result.unwrap();
    assert_eq!((app.path(E01).exists(), app.path(E01_SUBTITLE).exists()), (false, false));
    assert!(app.path("tv/Frieren (2023)/Season 01").exists());
    assert!(app.db_files().await.is_empty());
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::User]);
}
