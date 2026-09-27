mod common;

use std::{fs, os::unix::fs::PermissionsExt};

use common::{App, TODAY, frieren_metadata, now};
use yokoku_domain::{ItemFolder, ItemId, MonitorPreset, Series};
use yokoku_events::{DeleteReason, Event, FileDeleted, Handler, SeriesAdded, SeriesRemoved};
use yokoku_library::ports::SeriesRepo;
use yokoku_media::MediaError;

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv";
const E01_SUBTITLE: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.en.srt";
const E02: &str = "tv/Frieren (2023)/Season 02/Frieren (2023) - S02E01-E02.mkv";

async fn linked(app: &App, paths: &[&str]) {
    for path in paths {
        app.write(path, 10);
    }
    app.scanner.scan().await.unwrap();
}

fn deleted_events(events: &[Event]) -> Vec<DeleteReason> {
    events.iter().filter_map(Event::get::<FileDeleted>).map(|deleted| deleted.reason).collect()
}

#[tokio::test]
async fn deleting_removes_the_file_its_subtitles_and_empty_folders() {
    let app = App::new().await;
    linked(&app, &[E01, E01_SUBTITLE, E02]).await;

    let deleted = app.deleter().delete(app.episodes(1, 1, 1)).await.unwrap();

    assert_eq!(deleted.len(), 1);
    assert!(!app.path(E01).exists() && !app.path(E01_SUBTITLE).exists());
    assert!(!app.path("tv/Frieren (2023)/Season 01").exists());
    assert!(app.path(E02).exists());
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::User]);
}

#[tokio::test]
async fn one_episode_deletes_the_multi_episode_file_holding_it() {
    let app = App::new().await;
    linked(&app, &[E02]).await;

    app.deleter().delete(app.episodes(2, 2, 2)).await.unwrap();

    assert!(!app.path(E02).exists());
    assert!(app.db_files().await.is_empty());
}

#[tokio::test]
async fn deleting_what_has_no_file_fails() {
    let app = App::new().await;

    let error = app.deleter().delete(app.movie()).await.unwrap_err();

    assert!(matches!(error, MediaError::NoFile), "{error}");
}

#[tokio::test]
async fn removing_a_series_with_its_files_deletes_them_all() {
    let app = App::new().await;
    linked(&app, &[E01, E02, "movies/Dune (2021)/Dune (2021).mkv"]).await;
    let removed = |delete_files| SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files };
    let deleter = app.deleter();

    deleter.handle(&removed(true)).await.unwrap();
    deleter.handle(&removed(true)).await.unwrap();

    assert!(!app.path(E01).exists() && !app.path(E02).exists());
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::ItemRemoved; 2]);
}

#[tokio::test]
async fn files_kept_when_removing_a_series_return_when_it_is_added_again() {
    let app = App::new().await;
    linked(&app, &[E01, E02, "movies/Dune (2021)/Dune (2021).mkv"]).await;
    SeriesRepo::remove(&app.db, app.frieren.id).await.unwrap();

    app.deleter()
        .handle(&SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files: false })
        .await
        .unwrap();
    let folder = ItemFolder::new(app.path("tv"), "Frieren (2023)".into()).unwrap();
    let mut readded = Series::add(frieren_metadata(), folder, MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&app.db, &mut readded).await.unwrap();
    app.scanner.handle(&SeriesAdded { series: readded.id, title: "Frieren".into() }).await.unwrap();

    assert!(app.path(E01).exists() && app.path(E02).exists());
    let series_files: Vec<_> =
        app.db_files().await.into_iter().filter(|file| file.target.item() != ItemId::Movie(app.dune.id)).collect();
    assert_eq!(series_files.len(), 2);
    assert!(series_files.iter().all(|file| file.target.item() == ItemId::Series(readded.id)));
    assert!(deleted_events(&app.events().await).is_empty());
}

#[tokio::test]
async fn a_folder_that_cannot_be_removed_does_not_keep_the_deleted_file() {
    let app = App::new().await;
    linked(&app, &[E01, E01_SUBTITLE]).await;
    let series = app.path("tv/Frieren (2023)");
    fs::set_permissions(&series, fs::Permissions::from_mode(0o555)).unwrap();

    let result = app.deleter().delete(app.episodes(1, 1, 1)).await;

    fs::set_permissions(&series, fs::Permissions::from_mode(0o755)).unwrap();
    result.unwrap();
    assert!(!app.path(E01).exists() && !app.path(E01_SUBTITLE).exists());
    assert!(app.path("tv/Frieren (2023)/Season 01").exists());
    assert!(app.db_files().await.is_empty());
    assert_eq!(deleted_events(&app.events().await), [DeleteReason::User]);
}
