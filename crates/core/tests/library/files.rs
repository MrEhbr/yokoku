use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use yokoku_core::{
    events::Handler,
    library::{FileTracker, ports::MediaFiles},
};
use yokoku_domain::{
    EpisodeSpan, ExternalId, FileTarget, MediaFileId, MonitorPreset, Movie, Releases, Series, SeriesId, SourceStatus,
    StorageError,
    events::{DeleteReason, FileDeleted, FilesFound, FilesImported, LinkedFile},
};

use crate::common;

/// Media's library files, as the tests place and delete them.
#[derive(Default)]
struct Files(Mutex<HashMap<MediaFileId, FileTarget>>);

#[async_trait]
impl MediaFiles for Files {
    async fn target(&self, file: MediaFileId) -> Result<Option<FileTarget>, StorageError> {
        Ok(self.0.lock().unwrap().get(&file).copied())
    }
}

struct Setup {
    app: App,
    files: Arc<Files>,
    tracker: FileTracker,
    series: Series,
    movie: Movie,
}

impl Setup {
    fn place(&self, file: MediaFileId, target: FileTarget) {
        self.files.0.lock().unwrap().insert(file, target);
    }

    fn remove(&self, file: MediaFileId) {
        self.files.0.lock().unwrap().remove(&file);
    }

    /// Places `file` and delivers the `FilesFound` for it.
    async fn find(&self, file: MediaFileId, target: FileTarget) {
        self.place(file, target);
        self.tracker.handle(&found(file, target)).await.unwrap();
    }

    /// Removes `file` and delivers the `FileDeleted` for it.
    async fn delete(&self, file: MediaFileId, target: FileTarget) {
        self.remove(file);
        self.tracker.handle(&deleted(file, target)).await.unwrap();
    }
}

async fn setup() -> Setup {
    let app = App::new().await;
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[Some(TODAY); 3])]));
    app.provider.put_movie(movie_metadata(10, "Dune", Releases::default()));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app.metadata.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    let repo = Arc::new(app.db.clone());
    let files = Arc::new(Files::default());
    let tracker = FileTracker::new(repo.clone(), repo, files.clone());
    Setup { app, files, tracker, series, movie }
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

    setup.find(episode_file, episodes(setup.series.id, 1, 2)).await;
    setup.find(movie_file, FileTarget::Movie(setup.movie.id)).await;

    assert_eq!(episode_files(&setup).await, [Some(episode_file), Some(episode_file), None]);
    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, Some(movie_file));
}

#[tokio::test]
async fn imported_files_are_linked_like_found_ones() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let target = episodes(setup.series.id, 3, 3);
    setup.place(file, target);
    let event = FilesImported {
        import: yokoku_domain::ImportId::generate(),
        download: None,
        files: vec![LinkedFile { file, path: "/media/file.mkv".into(), target }],
        sources: Vec::new(),
    };

    setup.tracker.handle(&event).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn deleting_a_file_unlinks_only_that_file() {
    let setup = setup().await;
    let (old, new) = (MediaFileId::generate(), MediaFileId::generate());
    setup.find(old, episodes(setup.series.id, 1, 2)).await;
    setup.find(new, episodes(setup.series.id, 2, 2)).await;

    setup.delete(old, episodes(setup.series.id, 1, 2)).await;

    assert_eq!(episode_files(&setup).await, [None, Some(new), None]);
}

#[tokio::test]
async fn redelivered_events_change_nothing() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let movie = FileTarget::Movie(setup.movie.id);
    setup.find(file, movie).await;
    setup.delete(file, movie).await;

    setup.tracker.handle(&found(file, movie)).await.unwrap();
    setup.tracker.handle(&deleted(file, movie)).await.unwrap();

    assert_eq!(setup.app.library.movie(setup.movie.id).await.unwrap().file, None);
}

#[tokio::test]
async fn a_retried_link_of_a_file_deleted_since_is_skipped() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    let target = episodes(setup.series.id, 1, 1);
    setup.place(file, target);

    setup.delete(file, target).await;
    setup.tracker.handle(&found(file, target)).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, None]);
}

#[tokio::test]
async fn a_file_is_linked_where_media_holds_it_now() {
    let setup = setup().await;
    let file = MediaFileId::generate();
    setup.place(file, episodes(setup.series.id, 3, 3));

    setup.tracker.handle(&found(file, episodes(setup.series.id, 1, 1))).await.unwrap();

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn files_of_removed_items_and_missing_episodes_are_skipped() {
    let setup = setup().await;
    let (orphan, file) = (MediaFileId::generate(), MediaFileId::generate());

    setup.find(orphan, episodes(SeriesId::generate(), 1, 1)).await;
    setup.find(file, episodes(setup.series.id, 3, 9)).await;

    assert_eq!(episode_files(&setup).await, [None, None, Some(file)]);
}

#[tokio::test]
async fn file_links_survive_concurrent_refreshes() {
    let setup = setup().await;
    let files: Vec<MediaFileId> = (0..3).map(|_| MediaFileId::generate()).collect();

    let link = async {
        for (number, file) in (1..).zip(&files) {
            setup.find(*file, episodes(setup.series.id, number, number)).await;
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
