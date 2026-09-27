mod common;

use common::{App, relative};
use yokoku_domain::{EpisodeSpan, MediaFileId};
use yokoku_events::{EpisodesRenumbered, Event, Handler, ImportNeedsReview, RenumberedFile};
use yokoku_media::{ImportStatus, ports::MediaRepo};

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv";
const E01_E02: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01-E02.mkv";

async fn linked(app: &App, path: &str) -> MediaFileId {
    app.write(path, 10);
    app.scanner.scan().await.unwrap();
    app.db_files().await[0].id
}

#[tokio::test]
async fn a_renumbered_file_follows_its_episodes() {
    let app = App::new().await;
    let file = linked(&app, E01).await;
    let gone = RenumberedFile { file: MediaFileId::generate(), span: EpisodeSpan::new(2, 1, 1) };

    let event = EpisodesRenumbered {
        series: app.frieren.id,
        files: vec![RenumberedFile { file, span: EpisodeSpan::new(2, 3, 3) }, gone],
    };
    app.scanner.handle(&event).await.unwrap();

    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01]);
    assert_eq!(files[0].target, app.episodes(2, 3, 3));
}

#[tokio::test]
async fn a_file_whose_episodes_split_goes_to_review() {
    let app = App::new().await;
    let file = linked(&app, E01_E02).await;

    let event = EpisodesRenumbered { series: app.frieren.id, files: vec![RenumberedFile { file, span: None }] };
    app.scanner.handle(&event).await.unwrap();
    app.scanner.scan().await.unwrap();

    assert!(app.path(E01_E02).exists());
    assert!(app.db_files().await.is_empty());
    let review = app.db.imports(ImportStatus::NeedsReview).await.unwrap();
    assert_eq!(review.len(), 1);
    assert_eq!(relative(&app, review[0].rows.iter().map(|row| row.path.as_path())), [E01_E02]);
    assert_eq!(review[0].rows[0].target, None);
    let needs_review: Vec<_> = app.events().await.iter().filter_map(Event::get::<ImportNeedsReview>).cloned().collect();
    assert_eq!(needs_review, [ImportNeedsReview { import: review[0].id, source: review[0].source.clone() }]);
}
