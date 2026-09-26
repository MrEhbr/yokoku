mod common;

use std::sync::Arc;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::Timestamp;
use yokoku_domain::{
    EpisodeSpan, ExternalId, FileTarget, MediaFileId, MonitorPreset, Movie, Releases, Series, SeriesId, SourceStatus,
};
use yokoku_events::{DeleteReason, Event, EventId, LinkedFile, Recorded, Subscriber};
use yokoku_library::FileTracker;

struct Setup {
    app: App,
    tracker: FileTracker,
    series: Series,
    movie: Movie,
}

async fn setup() -> Setup {
    let app = App::new().await;
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[Some(TODAY); 3])]));
    app.metadata.put_movie(movie_metadata(10, "Dune", Releases::default()));
    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app.sync.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    let repo = Arc::new(app.db.clone());
    let tracker = FileTracker::new(repo.clone(), repo);
    Setup { app, tracker, series, movie }
}

fn recorded(event: Event) -> Recorded {
    Recorded { id: EventId(1), occurred_at: Timestamp::UNIX_EPOCH, event }
}

fn episodes(series: SeriesId, first: u16, last: u16) -> FileTarget {
    FileTarget::Episodes { series, span: EpisodeSpan::new(1, first, last).unwrap() }
}

fn found(file: MediaFileId, target: FileTarget) -> Event {
    Event::FilesFound { files: vec![LinkedFile { file, path: "/media/file.mkv".into(), target }] }
}

fn deleted(file: MediaFileId, target: FileTarget) -> Event {
    Event::FileDeleted { file, path: "/media/file.mkv".into(), target, reason: DeleteReason::External }
}

async fn episode_files(setup: &Setup) -> Vec<Option<MediaFileId>> {
    let series = setup.app.library.series(setup.series.id).await.unwrap();
    series.episodes().map(|episode| episode.file).collect()
}

#[tokio::test]
async fn found_files_link_every_episode_they_hold_and_movies() {
    let setup = setup().await;
    let (episode_file, movie_file) = (MediaFileId::generate(), MediaFileId::generate());

    setup.tracker.handle(&recorded(found(episode_file, episodes(setup.series.id, 1, 2)))).await.unwrap();
    setup.tracker.handle(&recorded(found(movie_file, FileTarget::Movie(setup.movie.id)))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [Some(episode_file), Some(episode_file), None]);
    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, Some(movie_file));
}

#[tokio::test]
async fn imported_files_are_linked_like_found_ones() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let event = Event::FilesImported {
        import: yokoku_domain::ImportId::generate(),
        download: None,
        files: vec![LinkedFile { file, path: "/media/file.mkv".into(), target: episodes(setup.series.id, 3, 3) }],
    };

    setup.tracker.handle(&recorded(event)).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn deleting_a_file_unlinks_only_that_file() {
    let setup = setup().await;
    let (old, new) = (MediaFileId::generate(), MediaFileId::generate());
    setup.tracker.handle(&recorded(found(old, episodes(setup.series.id, 1, 2)))).await.unwrap();
    setup.tracker.handle(&recorded(found(new, episodes(setup.series.id, 2, 2)))).await.unwrap();

    setup.tracker.handle(&recorded(deleted(old, episodes(setup.series.id, 1, 2)))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, Some(new), None]);
}

#[tokio::test]
async fn redelivered_events_change_nothing() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let events = [found(file, FileTarget::Movie(setup.movie.id)), deleted(file, FileTarget::Movie(setup.movie.id))];

    for event in events.iter().chain(&events) {
        setup.tracker.handle(&recorded(event.clone())).await.unwrap();
    }

    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, None);
}

#[tokio::test]
async fn files_of_removed_items_and_missing_episodes_are_skipped() {
    let setup = setup().await;
    let file = MediaFileId::generate();

    setup.tracker.handle(&recorded(found(file, episodes(SeriesId::generate(), 1, 1)))).await.unwrap();
    setup.tracker.handle(&recorded(found(file, episodes(setup.series.id, 3, 9)))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn file_links_survive_concurrent_refreshes() {
    let setup = setup().await;
    let files: Vec<MediaFileId> = (0..3).map(|_| MediaFileId::generate()).collect();

    let link = async {
        for (number, file) in (1..).zip(&files) {
            setup.tracker.handle(&recorded(found(*file, episodes(setup.series.id, number, number)))).await.unwrap();
        }
    };
    let refresh = async {
        for _ in 0..20 {
            setup.app.sync.refresh_series(setup.series.id).await.unwrap();
        }
    };
    tokio::join!(link, refresh);

    assert_eq!(episode_files(&setup).await, files.into_iter().map(Some).collect::<Vec<_>>());
}
