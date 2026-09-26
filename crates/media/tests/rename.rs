mod common;

use std::fs;

use common::{App, relative};
use yokoku_events::Event;
use yokoku_library::ports::MovieRepo;
use yokoku_media::{RenameScope, SkipReason, Skipped};

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv";
const MESSY: &str = "tv/frieren/Frieren (2023) - S01E01.mkv";

/// Links `path` to the library through a scan.
async fn linked(app: &App, path: &str) {
    app.write(path, 10);
    let report = app.scanner.scan().await.unwrap();
    assert_eq!(report.found, 1, "{path} was not linked");
}

#[tokio::test]
async fn preview_lists_moves_without_touching_disk() {
    let app = App::new().await;
    linked(&app, MESSY).await;
    app.write("tv/frieren/Frieren (2023) - S01E01.eng.srt", 1);

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames.len(), 1);
    let rename = &plan.renames[0];
    assert_eq!(relative(&app, [rename.video.from.as_path(), rename.video.to.as_path()]), [MESSY, E01]);
    assert_eq!(
        relative(&app, rename.subtitles.iter().map(|subtitle| subtitle.to.as_path())),
        ["tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.en.srt"]
    );
    assert!(app.path(MESSY).exists());
}

#[tokio::test]
async fn applying_moves_videos_and_subtitles_and_tidies_old_folders() {
    let app = App::new().await;
    linked(&app, MESSY).await;
    app.write("tv/frieren/Frieren (2023) - S01E01.eng.srt", 1);

    let report = app.renamer.apply(RenameScope::All).await.unwrap();

    assert_eq!(report.renamed.len(), 1);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(app.path(E01).exists());
    assert!(app.path("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.en.srt").exists());
    assert!(!app.path("tv/frieren").exists());
    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01]);
    assert_eq!(
        app.events().await.last(),
        Some(&Event::FileRenamed { file: files[0].id, from: app.path(MESSY), to: app.path(E01) })
    );
    assert_eq!(app.renamer.preview(RenameScope::All).await.unwrap().renames, []);
}

#[tokio::test]
async fn files_already_in_place_are_not_listed() {
    let app = App::new().await;
    linked(&app, E01).await;

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(plan.skipped, []);
}

#[tokio::test]
async fn a_scope_limits_the_plan_to_one_item() {
    let app = App::new().await;
    linked(&app, MESSY).await;
    linked(&app, "movies/dune.2021.mkv").await;

    let series = app.renamer.preview(RenameScope::Series(app.frieren.id)).await.unwrap();
    let movie = app.renamer.preview(RenameScope::Movie(app.dune.id)).await.unwrap();

    assert_eq!(relative(&app, series.renames.iter().map(|rename| rename.video.to.as_path())), [E01]);
    assert_eq!(
        relative(&app, movie.renames.iter().map(|rename| rename.video.to.as_path())),
        ["movies/Dune (2021)/Dune (2021).mkv"]
    );
}

#[tokio::test]
async fn files_without_a_root_or_a_library_item_are_skipped() {
    let app = App::new().await;
    linked(&app, MESSY).await;
    linked(&app, "movies/dune.2021.mkv").await;
    app.roots.remove(&app.path("tv")).await.unwrap();
    MovieRepo::remove(&app.db, app.dune.id, &[]).await.unwrap();

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(
        plan.skipped,
        [
            Skipped { path: app.path("movies/dune.2021.mkv"), reason: SkipReason::NotInLibrary },
            Skipped { path: app.path(MESSY), reason: SkipReason::OutsideRoots },
        ]
    );
}

#[tokio::test]
async fn an_unlinked_file_at_the_new_path_is_never_replaced() {
    let app = App::new().await;
    linked(&app, MESSY).await;
    let blocker = app.path(E01);
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, b"keep me").unwrap();

    let report = app.renamer.apply(RenameScope::All).await.unwrap();

    assert_eq!(report.renamed, []);
    assert_eq!(relative(&app, report.failed.iter().map(|failure| failure.path.as_path())), [MESSY]);
    assert_eq!(fs::read(&blocker).unwrap(), b"keep me");
    assert!(app.path(MESSY).exists());
    assert_eq!(relative(&app, app.db_files().await.iter().map(|file| file.path.as_path())), [MESSY]);
}

#[tokio::test]
async fn files_that_would_share_a_path_are_skipped() {
    let app = App::new().await;
    let remake = yokoku_domain::Movie {
        id: yokoku_domain::MovieId::generate(),
        source: yokoku_domain::ExternalId::Tmdb(1),
        ..app.dune.clone()
    };
    MovieRepo::save(&app.db, &remake, &[]).await.unwrap();
    app.write("movies/unsorted/a.mkv", 10);
    app.write("movies/unsorted/b.mkv", 10);
    let import = app.scanner.scan().await.unwrap().needs_review[0];
    app.review.match_row(import, 1, app.movie()).await.unwrap();
    app.review.match_row(import, 2, yokoku_domain::FileTarget::Movie(remake.id)).await.unwrap();
    app.review.approve(import).await.unwrap();

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(
        plan.skipped.iter().map(|skipped| skipped.reason).collect::<Vec<_>>(),
        [SkipReason::SharedTarget, SkipReason::SharedTarget]
    );
}
