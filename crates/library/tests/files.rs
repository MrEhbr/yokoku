mod common;

use std::sync::Arc;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use yokoku_domain::{
    EpisodeSpan, ExternalId, FileTarget, MediaFileId, MonitorPreset, Movie, Releases, Series, SeriesId, SourceStatus,
};
use yokoku_events::{DeleteReason, FileDeleted, FilesFound, FilesImported, Handler, LinkedFile};
use yokoku_library::FileTracker;

struct Setup {
    app: App,
    tracker: FileTracker,
    series: Series,
    movie: Movie,
}

async fn setup() -> Setup {
    let app = App::new().await;
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[Some(TODAY); 3])]));
    app.provider.put_movie(movie_metadata(10, "Dune", Releases::default()));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app.metadata.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    let repo = Arc::new(app.db.clone());
    let tracker = FileTracker::new(repo.clone(), repo);
    Setup { app, tracker, series, movie }
}

fn episodes(series: SeriesId, first: u16, last: u16) -> FileTarget {
    FileTarget::Episodes { series, span: EpisodeSpan::new(1, first, last).unwrap() }
}

fn found(file: MediaFileId, target: FileTarget) -> FilesFound {
    FilesFound { files: vec![LinkedFile { file, path: "/media/file.mkv".into(), target }] }
}

fn deleted(file: MediaFileId, target: FileTarget) -> FileDeleted {
    FileDeleted { file, path: "/media/file.mkv".into(), target, reason: DeleteReason::External }
}

async fn episode_files(setup: &Setup) -> Vec<Option<MediaFileId>> {
    let series = setup.app.library.series(setup.series.id).await.unwrap();
    series.episodes().map(|episode| episode.file).collect()
}

#[tokio::test]
async fn found_files_link_every_episode_they_hold_and_movies() {
    let setup = setup().await;
    let (episode_file, movie_file) = (MediaFileId::generate(), MediaFileId::generate());

    setup.tracker.handle(&found(episode_file, episodes(setup.series.id, 1, 2))).await.unwrap();
    setup.tracker.handle(&found(movie_file, FileTarget::Movie(setup.movie.id))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [Some(episode_file), Some(episode_file), None]);
    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, Some(movie_file));
}

#[tokio::test]
async fn imported_files_are_linked_like_found_ones() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let event = FilesImported {
        import: yokoku_domain::ImportId::generate(),
        download: None,
        files: vec![LinkedFile { file, path: "/media/file.mkv".into(), target: episodes(setup.series.id, 3, 3) }],
    };

    setup.tracker.handle(&event).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn deleting_a_file_unlinks_only_that_file() {
    let setup = setup().await;
    let (old, new) = (MediaFileId::generate(), MediaFileId::generate());
    setup.tracker.handle(&found(old, episodes(setup.series.id, 1, 2))).await.unwrap();
    setup.tracker.handle(&found(new, episodes(setup.series.id, 2, 2))).await.unwrap();

    setup.tracker.handle(&deleted(old, episodes(setup.series.id, 1, 2))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, Some(new), None]);
}

#[tokio::test]
async fn redelivered_events_change_nothing() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let movie = FileTarget::Movie(setup.movie.id);

    for _ in 0..2 {
        setup.tracker.handle(&found(file, movie)).await.unwrap();
        setup.tracker.handle(&deleted(file, movie)).await.unwrap();
    }

    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, None);
}

#[tokio::test]
async fn files_of_removed_items_and_missing_episodes_are_skipped() {
    let setup = setup().await;
    let file = MediaFileId::generate();

    setup.tracker.handle(&found(file, episodes(SeriesId::generate(), 1, 1))).await.unwrap();
    setup.tracker.handle(&found(file, episodes(setup.series.id, 3, 9))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn file_links_survive_concurrent_refreshes() {
    let setup = setup().await;
    let files: Vec<MediaFileId> = (0..3).map(|_| MediaFileId::generate()).collect();

    let link = async {
        for (number, file) in (1..).zip(&files) {
            setup.tracker.handle(&found(*file, episodes(setup.series.id, number, number))).await.unwrap();
        }
    };
    let refresh = async {
        for _ in 0..20 {
            setup.app.metadata.refresh_series(setup.series.id).await.unwrap();
        }
    };
    tokio::join!(link, refresh);

    assert_eq!(episode_files(&setup).await, files.into_iter().map(Some).collect::<Vec<_>>());
}
