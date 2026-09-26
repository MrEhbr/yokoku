mod common;

use std::path::Path;

use common::App;
use rstest::rstest;
use yokoku_media::{MediaError, RootFolder, RootKind};

#[tokio::test]
async fn added_root_folders_are_listed() {
    let app = App::new().await;

    let roots = app.roots.list().await.unwrap();

    assert_eq!(
        roots,
        [
            RootFolder { kind: RootKind::Movies, path: app.path("movies") },
            RootFolder { kind: RootKind::Series, path: app.path("tv") },
        ]
    );
}

#[tokio::test]
async fn trailing_separators_are_dropped() {
    let app = App::new().await;
    app.write("anime/.keep", 0);

    let root = app.roots.add(RootKind::Series, Path::new(&format!("{}/", app.path("anime").display()))).await.unwrap();

    assert_eq!(root.path, app.path("anime"));
}

#[rstest]
#[case::inside_another("tv/anime")]
#[case::around_another("")]
#[case::same_as_another("tv")]
#[tokio::test]
async fn root_folders_must_not_overlap(#[case] relative: &str) {
    let app = App::new().await;
    app.write("tv/anime/.keep", 0);

    let error = app.roots.add(RootKind::Series, &app.path(relative)).await.unwrap_err();

    assert!(matches!(error, MediaError::OverlappingRoot { .. }), "{error}");
}

#[tokio::test]
async fn root_folders_must_be_existing_absolute_folders() {
    let app = App::new().await;
    let file = app.write("notes.txt", 1);

    let relative = app.roots.add(RootKind::Series, Path::new("tv")).await.unwrap_err();
    let missing = app.roots.add(RootKind::Series, &app.path("missing")).await.unwrap_err();
    let not_a_folder = app.roots.add(RootKind::Series, &file).await.unwrap_err();

    assert!(matches!(relative, MediaError::RelativePath(_)), "{relative}");
    assert!(matches!(missing, MediaError::NotAFolder(_)), "{missing}");
    assert!(matches!(not_a_folder, MediaError::NotAFolder(_)), "{not_a_folder}");
}

#[tokio::test]
async fn removing_a_root_folder_forgets_it() {
    let app = App::new().await;

    app.roots.remove(&app.path("tv")).await.unwrap();
    let error = app.roots.remove(&app.path("tv")).await.unwrap_err();

    assert_eq!(app.roots.list().await.unwrap(), [RootFolder { kind: RootKind::Movies, path: app.path("movies") }]);
    assert!(matches!(error, MediaError::RootNotFound(_)), "{error}");
}
