use std::path::{Path, PathBuf};

use common::App;
use rstest::rstest;
use yokoku_core::media::{MediaError, RootFolder, RootKind};

use crate::common;

#[tokio::test]
async fn added_root_folders_are_listed() {
    let app = App::new().await;

    let roots = app.roots.list().await.unwrap();

    assert_eq!(
        roots,
        [
            RootFolder::new(RootKind::Movies, app.path("movies"), None, false),
            RootFolder::new(RootKind::Series, app.path("tv"), None, false),
        ]
    );
}

#[tokio::test]
async fn a_root_folders_folders_are_listed_by_name() {
    let app = App::new().await;
    std::fs::create_dir_all(app.path("tv/Frieren/Season 01")).unwrap();
    std::fs::create_dir(app.path("tv/Arcane (2021)")).unwrap();
    let root = RootFolder::new(RootKind::Series, app.path("tv"), None, false);

    let folders = app.roots.folders(&root).await.unwrap();

    assert_eq!(folders, ["Arcane (2021)", "Frieren"]);
}

#[tokio::test]
async fn a_root_folders_item_folders_are_listed_whether_they_exist_or_not() {
    let app = App::new().await;
    let tv = RootFolder::new(RootKind::Series, app.path("tv"), None, false);
    let anime = RootFolder::new(RootKind::Series, app.path("anime"), None, false);

    let taken = app.roots.item_folders(&tv).await.unwrap();
    let none = app.roots.item_folders(&anime).await.unwrap();

    assert_eq!(taken, ["Frieren (2023)"]);
    assert!(none.is_empty());
}

#[tokio::test]
async fn trailing_separators_are_dropped() {
    let app = App::new().await;
    app.write("anime/.keep", 0);

    let root =
        app.roots.add(RootKind::Series, Path::new(&format!("{}/", app.path("anime").display())), None).await.unwrap();

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

    let error = app.roots.add(RootKind::Series, &app.path(relative), None).await.unwrap_err();

    assert!(matches!(error, MediaError::OverlappingRoot { .. }), "{error}");
}

#[tokio::test]
async fn root_folders_must_be_existing_absolute_folders() {
    let app = App::new().await;
    let file = app.write("notes.txt", 1);

    let relative = app.roots.add(RootKind::Series, Path::new("tv"), None).await.unwrap_err();
    let missing = app.roots.add(RootKind::Series, &app.path("missing"), None).await.unwrap_err();
    let not_a_folder = app.roots.add(RootKind::Series, &file, None).await.unwrap_err();

    assert!(matches!(relative, MediaError::RelativePath(_)), "{relative}");
    assert!(matches!(missing, MediaError::NotAFolder(_)), "{missing}");
    assert!(matches!(not_a_folder, MediaError::NotAFolder(_)), "{not_a_folder}");
}

#[tokio::test]
async fn removing_a_root_folder_forgets_it() {
    let app = App::new().await;
    app.write("anime/.keep", 0);
    app.roots.add(RootKind::Series, &app.path("anime"), None).await.unwrap();

    app.roots.remove(&app.path("anime")).await.unwrap();
    let error = app.roots.remove(&app.path("anime")).await.unwrap_err();

    assert_eq!(app.roots.list().await.unwrap().len(), 2);
    assert!(matches!(error, MediaError::RootNotFound(_)), "{error}");
}

#[tokio::test]
async fn a_root_folder_holding_items_is_not_removed() {
    let app = App::new().await;

    let error = app.roots.remove(&app.path("tv")).await.unwrap_err();

    assert!(matches!(error, MediaError::RootInUse { items: 1, .. }), "{error}");
    assert_eq!(app.roots.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn items_are_added_to_a_root_folder_of_their_kind() {
    let app = App::new().await;

    let series = app.roots.get(RootKind::Series, &app.path("tv")).await.unwrap();
    let movies = app.roots.get(RootKind::Series, &app.path("movies")).await.unwrap_err();
    let missing = app.roots.get(RootKind::Series, &app.path("anime")).await.unwrap_err();

    assert_eq!(series, RootFolder::new(RootKind::Series, app.path("tv"), None, false));
    assert!(matches!(movies, MediaError::WrongRootKind { kind: RootKind::Movies, .. }), "{movies}");
    assert!(matches!(missing, MediaError::RootNotFound(_)), "{missing}");
}

fn configured(kind: RootKind, path: PathBuf, name: Option<&str>) -> RootFolder {
    RootFolder::new(kind, path, name.map(str::to_owned), true)
}

#[tokio::test]
async fn configured_root_folders_are_listed_with_the_stored_ones() {
    let app = App::new().await;
    let anime = configured(RootKind::Series, app.path("anime"), Some("Anime"));

    let roots = app.roots_with(vec![anime.clone()]).list().await.unwrap();

    assert_eq!(
        roots,
        [
            anime,
            RootFolder::new(RootKind::Movies, app.path("movies"), None, false),
            RootFolder::new(RootKind::Series, app.path("tv"), None, false),
        ]
    );
}

#[tokio::test]
async fn a_root_folder_both_configured_and_stored_is_listed_once_as_configured() {
    let app = App::new().await;
    let tv = configured(RootKind::Series, app.path("tv"), Some("Shows"));

    let roots = app.roots_with(vec![tv.clone()]).list().await.unwrap();

    assert_eq!(roots.iter().filter(|root| root.path == app.path("tv")).collect::<Vec<_>>(), [&tv]);
}

#[tokio::test]
async fn configured_root_folders_are_not_removed() {
    let app = App::new().await;
    let roots = app.roots_with(vec![configured(RootKind::Series, app.path("anime"), None)]);

    let error = roots.remove(&app.path("anime")).await.unwrap_err();

    assert!(matches!(error, MediaError::ConfiguredRoot(_)), "{error}");
}

#[tokio::test]
async fn added_root_folders_must_not_overlap_configured_ones() {
    let app = App::new().await;
    app.write("anime/new/.keep", 0);
    let roots = app.roots_with(vec![configured(RootKind::Series, app.path("anime"), None)]);

    let error = roots.add(RootKind::Series, &app.path("anime/new"), None).await.unwrap_err();

    assert!(matches!(error, MediaError::OverlappingRoot { .. }), "{error}");
}

#[rstest]
#[case::same_path_other_kind(RootKind::Movies, "tv")]
#[case::inside_a_stored_one(RootKind::Series, "tv/anime")]
#[case::around_a_stored_one(RootKind::Series, "")]
#[tokio::test]
async fn configured_root_folders_must_not_conflict_with_stored_ones(#[case] kind: RootKind, #[case] relative: &str) {
    let app = App::new().await;
    let roots = app.roots_with(vec![configured(kind, app.path(relative), None)]);

    let error = roots.check().await.unwrap_err();

    assert!(matches!(error, MediaError::ConflictingRoot { .. }), "{error}");
}

#[rstest]
#[case::relative(&["anime"])]
#[case::overlapping(&["/media/anime", "/media/anime/old"])]
#[tokio::test]
async fn configured_root_folders_must_be_absolute_and_apart(#[case] paths: &[&str]) {
    let app = App::new().await;
    let roots = app.roots_with(paths.iter().map(|path| configured(RootKind::Series, path.into(), None)).collect());

    assert!(roots.check().await.is_err());
}

#[tokio::test]
async fn a_root_folder_in_both_places_with_the_same_kind_does_not_conflict() {
    let app = App::new().await;
    let roots = app.roots_with(vec![configured(RootKind::Series, app.path("tv"), None)]);

    roots.check().await.unwrap();
}

#[tokio::test]
async fn added_root_folders_keep_their_name() {
    let app = App::new().await;
    app.write("CartoonShows/.keep", 0);

    app.roots.add(RootKind::Series, &app.path("CartoonShows"), Some("Cartoon shows".into())).await.unwrap();
    let root = app.roots.get(RootKind::Series, &app.path("CartoonShows")).await.unwrap();

    assert_eq!(root.name, "Cartoon shows");
}

#[tokio::test]
async fn a_root_folder_without_a_name_is_named_by_its_folder() {
    let root = RootFolder::new(RootKind::Series, "/media/library/Anime".into(), None, false);

    assert_eq!(root.name, "Anime");
}
