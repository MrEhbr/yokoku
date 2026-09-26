mod common;

use std::fs;

use common::App;
use jiff::Timestamp;
use yokoku_events::{DeleteReason, Event, EventId, Recorded, Subscriber};
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

fn deleted_events(events: &[Event]) -> Vec<(DeleteReason, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::FileDeleted { reason, recycled, .. } => Some((*reason, *recycled)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn deleting_removes_the_file_its_subtitles_and_empty_folders() {
    let app = App::new().await;
    linked(&app, &[E01, E01_SUBTITLE, E02]).await;

    let deleted = app.deleter(None).delete(app.episodes(1, 1, 1)).await.unwrap();

    assert_eq!(deleted.len(), 1);
    assert!(!app.path(E01).exists() && !app.path(E01_SUBTITLE).exists());
    assert!(!app.path("tv/Frieren (2023)/Season 01").exists());
    assert!(app.path(E02).exists());
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [(DeleteReason::User, false)]);
}

#[tokio::test]
async fn one_episode_deletes_the_multi_episode_file_holding_it() {
    let app = App::new().await;
    linked(&app, &[E02]).await;

    app.deleter(None).delete(app.episodes(2, 2, 2)).await.unwrap();

    assert!(!app.path(E02).exists());
    assert!(app.db_files().await.is_empty());
}

#[tokio::test]
async fn recycled_files_keep_their_place_under_a_folder_for_the_day() {
    let app = App::new().await;
    linked(&app, &[E01, E01_SUBTITLE]).await;

    app.deleter(Some(app.recycle(30))).delete(app.episodes(1, 1, 1)).await.unwrap();

    let day = app.path("recycle/2026-09-26");
    assert!(day.join(E01).exists() && day.join(E01_SUBTITLE).exists());
    assert!(!app.path(E01).exists());
    assert_eq!(deleted_events(&app.events().await), [(DeleteReason::User, true)]);
}

#[tokio::test]
async fn a_second_recycled_file_at_the_same_path_goes_under_its_id() {
    let app = App::new().await;
    let deleter = app.deleter(Some(app.recycle(30)));
    linked(&app, &[E01]).await;
    deleter.delete(app.episodes(1, 1, 1)).await.unwrap();
    linked(&app, &[E01]).await;
    let second = app.db_files().await.remove(0);

    deleter.delete(app.episodes(1, 1, 1)).await.unwrap();

    let day = app.path("recycle/2026-09-26");
    assert!(day.join(E01).exists());
    assert!(day.join(second.id.to_string()).join(E01).exists());
}

#[tokio::test]
async fn deleting_what_has_no_file_fails() {
    let app = App::new().await;

    let error = app.deleter(None).delete(app.movie()).await.unwrap_err();

    assert!(matches!(error, MediaError::NoFile), "{error}");
}

#[tokio::test]
async fn removing_a_series_with_its_files_deletes_them_all() {
    let app = App::new().await;
    linked(&app, &[E01, E02, "movies/Dune (2021)/Dune (2021).mkv"]).await;
    let removed = |delete_files| Recorded {
        id: EventId(1),
        occurred_at: Timestamp::UNIX_EPOCH,
        event: Event::SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files },
    };
    let deleter = app.deleter(None);

    deleter.handle(&removed(false)).await.unwrap();
    assert_eq!(app.db_files().await.len(), 3);
    deleter.handle(&removed(true)).await.unwrap();
    deleter.handle(&removed(true)).await.unwrap();

    assert!(!app.path(E01).exists() && !app.path(E02).exists());
    assert_eq!(app.db_files().await.len(), 1);
    assert_eq!(deleted_events(&app.events().await), [(DeleteReason::ItemRemoved, false); 2]);
}

#[tokio::test]
async fn cleaning_removes_day_folders_older_than_the_kept_days() {
    let app = App::new().await;
    for folder in
        ["recycle/2026-08-01/a.mkv", "recycle/2026-08-27/b.mkv", "recycle/2026-08-26/c.mkv", "recycle/notes/d.txt"]
    {
        app.write(folder, 1);
    }

    let removed = app.deleter(Some(app.recycle(30))).clean_recycle().await.unwrap();

    assert_eq!(removed, 2);
    let mut left: Vec<_> = fs::read_dir(app.path("recycle")).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    left.sort();
    assert_eq!(left, ["2026-08-27", "notes"]);
    assert_eq!(app.deleter(None).clean_recycle().await.unwrap(), 0);
}
