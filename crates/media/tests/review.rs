mod common;

use common::App;
use rstest::rstest;
use yokoku_detect::Conflict;
use yokoku_domain::{Confidence, DownloadId, EpisodeSpan, FileTarget, ImportId, MovieId, SeriesId};
use yokoku_events::{Event, LinkedFile};
use yokoku_media::{
    Approval, Import, ImportRow, ImportStatus, MediaError,
    ports::{Changes, MediaRepo},
};

/// Scans three unrecognised files in one folder and returns their import.
async fn pending(app: &App) -> ImportId {
    for name in ["a", "b", "c"] {
        app.write(&format!("tv/Unsorted/{name}.mkv"), 10);
    }
    let report = app.scanner.scan().await.unwrap();
    report.needs_review[0]
}

#[tokio::test]
async fn approving_links_matched_rows_and_leaves_skipped_ones() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.review.match_row(id, 1, app.episodes(1, 1, 2)).await.unwrap();
    app.review.match_row(id, 2, app.movie()).await.unwrap();
    app.review.skip_row(id, 3).await.unwrap();

    let Approval::Linked(files) = app.review.approve(id).await.unwrap() else { panic!("scan imports are linked") };

    assert_eq!(
        files.iter().map(|file| (file.path.clone(), file.target)).collect::<Vec<_>>(),
        [(app.path("tv/Unsorted/a.mkv"), app.episodes(1, 1, 2)), (app.path("tv/Unsorted/b.mkv"), app.movie()),]
    );
    assert_eq!(app.db_files().await.len(), 2);
    let linked = files.iter().map(|file| LinkedFile { file: file.id, path: file.path.clone(), target: file.target });
    assert_eq!(
        app.events().await.last(),
        Some(&Event::FilesImported { import: id, download: None, files: linked.collect() })
    );
    assert!(app.review.pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn approved_and_skipped_files_are_not_reviewed_again() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.review.match_row(id, 1, app.episodes(1, 1, 1)).await.unwrap();
    app.review.skip_row(id, 2).await.unwrap();
    app.review.skip_row(id, 3).await.unwrap();
    app.review.approve(id).await.unwrap();

    let report = app.scanner.scan().await.unwrap();

    assert!(report.needs_review.is_empty());
}

#[tokio::test]
async fn every_row_needs_a_match_or_a_skip() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.review.match_row(id, 2, app.movie()).await.unwrap();

    let error = app.review.approve(id).await.unwrap_err();

    assert!(matches!(error, MediaError::UnmatchedRows(ref rows) if rows == &[1, 3]), "{error}");
    assert!(app.db_files().await.is_empty());
}

#[tokio::test]
async fn rows_sharing_an_episode_or_holding_a_linked_one_conflict() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E03.mkv", 10);
    let id = pending(&app).await;
    app.review.match_row(id, 1, app.episodes(1, 1, 2)).await.unwrap();
    app.review.match_row(id, 2, app.episodes(1, 2, 2)).await.unwrap();
    app.review.match_row(id, 3, app.episodes(1, 3, 3)).await.unwrap();

    let review = app.review.get(id).await.unwrap();
    let error = app.review.approve(id).await.unwrap_err();

    let conflicts: Vec<_> = review.rows.iter().map(|row| row.conflicts.clone()).collect();
    assert_eq!(conflicts, [vec![Conflict::SharedTarget], vec![Conflict::SharedTarget], vec![Conflict::AlreadyHasFile]]);
    assert!(matches!(error, MediaError::ConflictingRows(ref rows) if rows == &[1, 2, 3]), "{error}");
}

#[tokio::test]
async fn skipping_a_row_clears_its_conflicts() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.review.match_row(id, 1, app.movie()).await.unwrap();
    app.review.match_row(id, 2, app.movie()).await.unwrap();
    app.review.skip_row(id, 2).await.unwrap();
    app.review.skip_row(id, 3).await.unwrap();

    let review = app.review.get(id).await.unwrap();

    assert!(review.rows.iter().all(|row| row.conflicts.is_empty()));
    app.review.approve(id).await.unwrap();
}

#[rstest]
#[case::missing_episode(|app: &App| app.episodes(1, 3, 4), "S01E03-E04 is not in the series")]
#[case::missing_series(|_: &App| FileTarget::Episodes { series: SeriesId::generate(), span: EpisodeSpan::new(1, 1, 1).unwrap() }, "is not in the library")]
#[case::missing_movie(|_: &App| FileTarget::Movie(MovieId::generate()), "is not in the library")]
#[tokio::test]
async fn matches_must_be_in_the_library(#[case] target: fn(&App) -> FileTarget, #[case] message: &str) {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.review.match_row(id, 1, target(&app)).await.unwrap_err();

    assert!(error.to_string().contains(message), "{error}");
    assert_eq!(app.review.get(id).await.unwrap().rows[0].row.target, None);
}

#[rstest]
#[case(0)]
#[case(4)]
#[tokio::test]
async fn rows_are_numbered_from_one(#[case] row: usize) {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.review.skip_row(id, row).await.unwrap_err();

    assert!(matches!(error, MediaError::RowNotFound(number) if number == row), "{error}");
}

#[tokio::test]
async fn done_and_unknown_imports_cannot_be_reviewed() {
    let app = App::new().await;
    let id = pending(&app).await;
    for row in 1..=3 {
        app.review.skip_row(id, row).await.unwrap();
    }
    app.review.approve(id).await.unwrap();

    let done = app.review.approve(id).await.unwrap_err();
    let unknown = app.review.get(ImportId::generate()).await.unwrap_err();

    assert!(matches!(done, MediaError::NotInReview(_)), "{done}");
    assert!(matches!(unknown, MediaError::ImportNotFound(_)), "{unknown}");
    let stored = yokoku_media::ports::MediaRepo::import(&app.db, id).await.unwrap().unwrap();
    assert_eq!(stored.status, ImportStatus::Done);
}

/// An import of a download with one file, matched to S01E01.
async fn downloaded(app: &App) -> ImportId {
    let import = Import {
        id: ImportId::generate(),
        source: app.path("downloads/Frieren.S01E01"),
        download: Some(DownloadId::generate()),
        status: ImportStatus::NeedsReview,
        error: None,
        rows: vec![ImportRow {
            path: app.path("downloads/Frieren.S01E01/Frieren.S01E01.mkv"),
            size: 10,
            target: Some(app.episodes(1, 1, 1)),
            confidence: Confidence::Certain,
            skipped: false,
            replace: false,
        }],
        created_at: common::now(),
    };
    MediaRepo::save(&app.db, &Changes { imports: vec![import.clone()], ..Changes::default() }, &[]).await.unwrap();
    import.id
}

#[tokio::test]
async fn approving_a_download_queues_it_for_placing() {
    let app = App::new().await;
    let id = downloaded(&app).await;

    let approval = app.review.approve(id).await.unwrap();

    assert_eq!(approval, Approval::Queued);
    let stored = MediaRepo::import(&app.db, id).await.unwrap().unwrap();
    assert_eq!(stored.status, ImportStatus::Approved);
    assert!(app.db_files().await.is_empty());
    assert!(app.events().await.is_empty());
}

#[tokio::test]
async fn a_download_row_can_replace_the_library_file() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv", 10);
    app.scanner.scan().await.unwrap();
    let id = downloaded(&app).await;
    let conflicts = |review: yokoku_media::ImportReview| review.rows[0].conflicts.clone();
    assert_eq!(conflicts(app.review.get(id).await.unwrap()), [Conflict::AlreadyHasFile]);

    app.review.replace_row(id, 1).await.unwrap();

    let review = app.review.get(id).await.unwrap();
    assert!(review.rows[0].row.replace);
    assert_eq!(conflicts(review), []);
    assert_eq!(app.review.approve(id).await.unwrap(), Approval::Queued);
}

#[tokio::test]
async fn files_found_by_a_scan_cannot_replace_library_files() {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.review.replace_row(id, 1).await.unwrap_err();

    assert!(matches!(error, MediaError::ReplaceInPlace), "{error}");
}
