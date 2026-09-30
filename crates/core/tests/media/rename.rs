use std::{fs, os::unix::fs::PermissionsExt};

use common::{App, now, relative};
use yokoku_core::{
    events::FileRenamed,
    library::ports::{MovieRepo, SeriesRepo},
    media::{
        MediaFile, RenameScope, SkipReason, Skipped,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::MediaFileId;

use crate::common;

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv";
const MESSY: &str = "tv/Frieren (2023)/S1/Frieren (2023) - S01E01.mkv";

#[tokio::test]
async fn preview_lists_moves_without_touching_disk() {
    let app = App::new().await;
    app.linked(&[MESSY]).await;
    app.write("tv/Frieren (2023)/S1/Frieren (2023) - S01E01.eng.srt", 1);

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
    app.linked(&[MESSY]).await;
    app.write("tv/Frieren (2023)/S1/Frieren (2023) - S01E01.eng.srt", 1);

    let report = app.renamer.apply(RenameScope::All, None).await.unwrap();

    assert_eq!(report.renamed.len(), 1);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(app.path(E01).exists());
    assert!(app.path("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.en.srt").exists());
    assert!(!app.path("tv/Frieren (2023)/S1").exists());
    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01]);
    assert_eq!(
        app.events().await.last(),
        Some(
            &FileRenamed { file: files[0].id, from: app.path(MESSY), to: app.path(E01), target: Some(files[0].target) }
                .into()
        )
    );
    assert_eq!(app.renamer.preview(RenameScope::All).await.unwrap().renames, []);
}

#[tokio::test]
async fn applying_to_chosen_files_leaves_the_others() {
    let app = App::new().await;
    app.linked(&[MESSY]).await;

    let report = app.renamer.apply(RenameScope::All, Some(&[MediaFileId::generate()])).await.unwrap();

    assert_eq!(report.renamed, []);
    assert!(app.path(MESSY).exists());
    assert_eq!(app.renamer.preview(RenameScope::All).await.unwrap().renames.len(), 1);
}

#[tokio::test]
async fn files_already_in_place_are_not_listed() {
    let app = App::new().await;
    app.linked(&[E01]).await;

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(plan.skipped, []);
}

#[tokio::test]
async fn a_scope_limits_the_plan_to_one_item() {
    let app = App::new().await;
    app.linked(&[MESSY]).await;
    app.linked(&["movies/Dune (2021)/dune.2021.mkv"]).await;

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
    app.linked(&[MESSY]).await;
    app.linked(&["movies/Dune (2021)/dune.2021.mkv"]).await;
    SeriesRepo::remove(&app.db, app.frieren.id).await.unwrap();
    app.roots.remove(&app.path("tv")).await.unwrap();
    MovieRepo::remove(&app.db, app.dune.id).await.unwrap();

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(
        plan.skipped,
        [
            Skipped { path: app.path("movies/Dune (2021)/dune.2021.mkv"), reason: SkipReason::NotInLibrary },
            Skipped { path: app.path(MESSY), reason: SkipReason::OutsideRoots },
        ]
    );
}

#[tokio::test]
async fn an_unlinked_file_at_the_new_path_is_never_replaced() {
    let app = App::new().await;
    app.linked(&[MESSY]).await;
    let blocker = app.path(E01);
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, b"keep me").unwrap();

    let report = app.renamer.apply(RenameScope::All, None).await.unwrap();

    assert_eq!(report.renamed, []);
    assert_eq!(relative(&app, report.failed.iter().map(|failure| failure.path.as_path())), [MESSY]);
    assert_eq!(fs::read(&blocker).unwrap(), b"keep me");
    assert!(app.path(MESSY).exists());
    assert_eq!(relative(&app, app.db_files().await.iter().map(|file| file.path.as_path())), [MESSY]);
}

#[tokio::test]
async fn files_that_would_share_a_path_are_skipped() {
    let app = App::new().await;
    let file = |name: &str| MediaFile {
        id: MediaFileId::generate(),
        path: app.write(&format!("movies/Dune (2021)/{name}.mkv"), 10),
        size: 10,
        target: app.movie(),
        added_at: now(),
    };
    let changes = Changes { added_files: vec![file("a"), file("b")], ..Changes::default() };
    MediaRepo::save(&app.db, &changes).await.unwrap();

    let plan = app.renamer.preview(RenameScope::All).await.unwrap();

    assert_eq!(plan.renames, []);
    assert_eq!(
        plan.skipped.iter().map(|skipped| skipped.reason).collect::<Vec<_>>(),
        [SkipReason::SharedTarget, SkipReason::SharedTarget]
    );
}

#[tokio::test]
async fn an_old_folder_that_cannot_be_removed_does_not_stop_the_other_renames() {
    let app = App::new().await;
    let second = "tv/Frieren (2023)/S2/Frieren (2023) - S02E01.mkv";
    app.write(MESSY, 10);
    app.write(second, 10);
    app.scanner.scan().await.unwrap();
    let series = app.path("tv/Frieren (2023)");
    fs::create_dir_all(series.join("Season 01")).unwrap();
    fs::create_dir_all(series.join("Season 02")).unwrap();
    fs::set_permissions(&series, fs::Permissions::from_mode(0o555)).unwrap();

    let result = app.renamer.apply(RenameScope::All, None).await;

    fs::set_permissions(&series, fs::Permissions::from_mode(0o755)).unwrap();
    let report = result.unwrap();
    assert_eq!(report.renamed.len(), 2);
    assert!(app.path(E01).exists());
    assert!(app.path("tv/Frieren (2023)/Season 02/Frieren (2023) - S02E01 - Episode 1.mkv").exists());
    assert!(app.path("tv/Frieren (2023)/S1").exists());
}
