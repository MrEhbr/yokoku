mod common;

use std::fs;

use common::{App, relative};
use yokoku_domain::{Confidence, ItemId};
use yokoku_events::{DeleteReason, FileDeleted, FilesFound, ImportNeedsReview, LinkedFile};
use yokoku_media::{ImportRow, ImportStatus, MediaFile, ScanReport};

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv";
const E02: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E02 - Episode 2.mkv";
const DUNE: &str = "movies/Dune (2021)/Dune (2021).mkv";

fn linked(files: &[MediaFile]) -> Vec<LinkedFile> {
    files.iter().map(|file| LinkedFile { file: file.id, path: file.path.clone(), target: file.target }).collect()
}

#[tokio::test]
async fn certain_matches_are_linked_where_they_are() {
    let app = App::new().await;
    app.write(E01, 10);
    app.write(DUNE, 30);
    app.write("movies/Dune (2021)/Sample/sample.mkv", 1);

    let report = app.scanner.scan().await.unwrap();

    let files = app.db_files().await;
    assert_eq!(report, ScanReport { found: 2, ..ScanReport::default() });
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [DUNE, E01]);
    assert_eq!(
        files.iter().map(|file| (file.target, file.size)).collect::<Vec<_>>(),
        [(app.movie(), 30), (app.episodes(1, 1, 1), 10)]
    );
    assert_eq!(
        app.events().await,
        [FilesFound { files: linked(&files[1..]) }.into(), FilesFound { files: linked(&files[..1]) }.into()]
    );
}

#[tokio::test]
async fn unsure_matches_go_to_review_per_item_folder() {
    let app = App::new().await;
    app.write(E01, 10);
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E09.mkv", 11);
    app.write("tv/Frieren (2023)/Frieren - 02.mkv", 12);

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report.found, 1);
    let pending = app.reviewer.pending().await.unwrap();
    assert_eq!(report.needs_review, pending.iter().map(|import| import.id).collect::<Vec<_>>());
    assert_eq!(relative(&app, pending.iter().map(|import| import.source.as_path())), ["tv/Frieren (2023)"]);
    let rows = |index: usize| -> Vec<(Option<_>, Confidence)> {
        pending[index].rows.iter().map(|row: &ImportRow| (row.target, row.confidence)).collect()
    };
    assert_eq!(rows(0), [(Some(app.episodes(1, 2, 2)), Confidence::Guess), (None, Confidence::Unknown)]);
    assert!(pending.iter().all(|import| import.status == ImportStatus::NeedsReview));
    let events = app.events().await;
    assert!(events.contains(&ImportNeedsReview { import: pending[0].id, source: pending[0].source.clone() }.into()));
}

#[tokio::test]
async fn files_outside_item_folders_are_ignored() {
    let app = App::new().await;
    app.write("tv/Home Videos/birthday.mkv", 13);
    app.write("tv/Frieren (2023) - S01E01.mkv", 10);
    app.write("movies/Dune (2021).mkv", 30);

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report, ScanReport::default());
    assert!(app.reviewer.pending().await.unwrap().is_empty());
    assert!(app.events().await.is_empty());
}

#[tokio::test]
async fn an_item_folder_not_on_disk_holds_no_files() {
    let app = App::new().await;

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report, ScanReport::default());
}

#[tokio::test]
async fn scanning_an_item_reads_only_its_folder() {
    let app = App::new().await;
    app.write(E01, 10);
    app.write(DUNE, 30);

    let report = app.scanner.scan_item(ItemId::Series(app.frieren.id)).await.unwrap();

    assert_eq!(report, ScanReport { found: 1, ..ScanReport::default() });
    assert_eq!(relative(&app, app.db_files().await.iter().map(|file| file.path.as_path())), [E01]);
}

#[tokio::test]
async fn a_second_file_for_a_linked_episode_goes_to_review() {
    let app = App::new().await;
    app.write(E01, 10);
    app.scanner.scan().await.unwrap();
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.1080p.mkv", 20);

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report.found, 0);
    let pending = app.reviewer.pending().await.unwrap();
    assert_eq!(pending[0].rows[0].target, Some(app.episodes(1, 1, 1)));
    assert_eq!(pending[0].rows[0].confidence, Confidence::Certain);
}

#[tokio::test]
async fn scanning_again_changes_nothing() {
    let app = App::new().await;
    app.write(E01, 10);
    app.write("tv/Frieren (2023)/Frieren - 02.mkv", 12);
    app.scanner.scan().await.unwrap();
    let events = app.events().await;

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report, ScanReport::default());
    assert_eq!(app.events().await, events);
    assert_eq!(app.reviewer.pending().await.unwrap().len(), 1);
}

#[tokio::test]
async fn files_gone_from_disk_are_forgotten() {
    let app = App::new().await;
    let gone = app.write(E01, 10);
    app.write(E02, 10);
    app.scanner.scan().await.unwrap();
    let before = app.db_files().await;
    fs::remove_file(&gone).unwrap();

    let report = app.scanner.scan().await.unwrap();

    assert_eq!(report, ScanReport { vanished: 1, ..ScanReport::default() });
    assert_eq!(app.db_files().await, before[1..]);
    assert_eq!(
        app.events().await.last(),
        Some(
            &FileDeleted {
                file: before[0].id,
                path: gone,
                target: app.episodes(1, 1, 1),
                reason: DeleteReason::External,
            }
            .into()
        )
    );
}

#[tokio::test]
async fn an_unreadable_root_stops_the_scan_and_forgets_nothing() {
    let app = App::new().await;
    app.write(E01, 10);
    app.scanner.scan().await.unwrap();
    let before = app.db_files().await;
    fs::remove_dir_all(app.path("tv")).unwrap();

    let error = app.scanner.scan().await.unwrap_err();

    assert!(error.to_string().contains("tv"), "{error}");
    assert_eq!(app.db_files().await, before);
}

#[tokio::test]
async fn a_scan_announces_the_imports_it_sends_to_review_and_not_the_files_it_links() {
    let app = App::new().await;
    app.write(E01, 10);
    let mut watch = app.changes.watch();
    app.scanner.scan().await.unwrap();
    let after_linking = watch.has_changed().unwrap();
    watch.borrow_and_update();
    app.write("tv/Frieren (2023)/Frieren - 02.mkv", 12);

    app.scanner.scan().await.unwrap();

    assert!(!after_linking);
    assert!(watch.has_changed().unwrap());
}
