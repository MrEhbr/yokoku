mod common;

use common::{App, relative};
use yokoku_domain::{Confidence, DownloadId, ItemId};
use yokoku_events::{DownloadCompleted, Handler, ImportFailed, ImportNeedsReview};
use yokoku_media::{ImportStatus, ports::MediaRepo};

#[tokio::test]
async fn a_certain_download_for_a_linked_series_is_approved() {
    let app = App::new().await;
    app.write("downloads/Frieren.S01.1080p/Frieren.S01E01.1080p.mkv", 10);
    app.write("downloads/Frieren.S01.1080p/Frieren.S01E02.1080p.mkv", 10);
    app.write("downloads/Frieren.S01.1080p/Sample/sample.mkv", 1);
    let item = Some(ItemId::Series(app.frieren.id));

    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path("downloads/Frieren.S01.1080p"), item)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(import.status, ImportStatus::Approved);
    assert_eq!(
        relative(&app, import.rows.iter().map(|row| row.path.as_path())),
        [
            "downloads/Frieren.S01.1080p/Frieren.S01E01.1080p.mkv",
            "downloads/Frieren.S01.1080p/Frieren.S01E02.1080p.mkv",
        ]
    );
    assert_eq!(
        import.rows.iter().map(|row| row.target).collect::<Vec<_>>(),
        [Some(app.episodes(1, 1, 1)), Some(app.episodes(1, 2, 2))]
    );
    assert!(app.events().await.is_empty());
}

#[tokio::test]
async fn an_unlinked_single_file_download_is_matched_against_the_library() {
    let app = App::new().await;
    let file = app.write("downloads/Dune.2021.1080p.BluRay.mkv", 10);

    let import = app.planner.plan(DownloadId::generate(), &file, None).await.unwrap().unwrap();

    assert_eq!((import.status, import.rows[0].target), (ImportStatus::Approved, Some(app.movie())));
    assert_eq!(import.source, file);
}

#[tokio::test]
async fn unsure_downloads_go_to_review() {
    let app = App::new().await;
    app.write("downloads/Frieren - 02 [1080p].mkv", 10);

    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path("downloads/Frieren - 02 [1080p].mkv"), None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(import.status, ImportStatus::NeedsReview);
    assert_eq!((import.rows[0].target, import.rows[0].confidence), (Some(app.episodes(1, 2, 2)), Confidence::Guess));
    assert_eq!(app.events().await, [ImportNeedsReview { import: import.id, source: import.source }.into()]);
}

#[tokio::test]
async fn a_download_of_an_episode_that_has_a_file_goes_to_review() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv", 10);
    app.scanner.scan().await.unwrap();
    let file = app.write("downloads/Frieren.S01E01.2160p.mkv", 20);

    let import =
        app.planner.plan(DownloadId::generate(), &file, Some(ItemId::Series(app.frieren.id))).await.unwrap().unwrap();

    assert_eq!(import.status, ImportStatus::NeedsReview);
}

#[tokio::test]
async fn a_download_without_videos_fails_visibly() {
    let app = App::new().await;
    app.write("downloads/Soundtrack/01.flac", 10);

    let import =
        app.planner.plan(DownloadId::generate(), &app.path("downloads/Soundtrack"), None).await.unwrap().unwrap();

    assert_eq!(import.status, ImportStatus::Failed);
    assert_eq!(
        app.events().await,
        [ImportFailed { import: import.id, source: import.source, reason: "the download holds no video files".into() }
            .into()]
    );
}

#[tokio::test]
async fn a_redelivered_completion_plans_once() {
    let app = App::new().await;
    let file = app.write("downloads/Dune.2021.1080p.mkv", 10);
    let download = DownloadId::generate();
    let completed = DownloadCompleted { download, name: "Dune.2021.1080p.mkv".into(), content_path: file, item: None };

    app.planner.handle(&completed).await.unwrap();
    app.planner.handle(&completed).await.unwrap();

    let import = app.db.import_for_download(download).await.unwrap().unwrap();
    assert_eq!(import.rows[0].target, Some(app.movie()));
    assert_eq!(app.db.imports(ImportStatus::Approved).await.unwrap().len(), 1);
}
