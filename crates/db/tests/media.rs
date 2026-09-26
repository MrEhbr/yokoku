use std::path::{Path, PathBuf};

use jiff::{Timestamp, ToSpan, tz::TimeZone};
use proptest::prelude::*;
use rstest::{fixture, rstest};
use uuid::Uuid;
use yokoku_db::Database;
use yokoku_domain::{
    Confidence, EpisodeSpan, ExternalId, FileTarget, ImportId, MediaFileId, MonitorPreset, Movie, MovieId,
    MovieMetadata, Releases, Series, SeriesId, SeriesMetadata, SourceStatus,
};
use yokoku_events::{DeleteReason, Event, EventLog};
use yokoku_library::ports::{MovieRepo, SeriesRepo};
use yokoku_media::{
    Import, ImportRow, ImportStatus, MediaFile, RootFolder, RootKind,
    ports::{Catalog, Changes, MediaRepo},
};

#[fixture]
async fn db() -> Database {
    Database::open_in_memory().await.unwrap()
}

fn now() -> Timestamp {
    "2026-09-26T12:00:00.123456789Z".parse().unwrap()
}

fn episodes(first: u16, last: u16) -> FileTarget {
    FileTarget::Episodes { series: SeriesId(Uuid::from_u128(1)), span: EpisodeSpan::new(1, first, last).unwrap() }
}

fn file(path: &str, target: FileTarget) -> MediaFile {
    MediaFile { id: MediaFileId::generate(), path: path.into(), size: 1 << 33, target, added_at: now() }
}

fn import(source: &str, created_at: Timestamp, rows: Vec<ImportRow>) -> Import {
    Import { id: ImportId::generate(), source: source.into(), status: ImportStatus::NeedsReview, rows, created_at }
}

fn row(path: &str, target: Option<FileTarget>) -> ImportRow {
    ImportRow { path: path.into(), size: 7, target, confidence: Confidence::Guess, skipped: false }
}

#[rstest]
#[tokio::test]
async fn root_folders_are_listed_by_path_and_removed_by_path(#[future] db: Database) {
    let db = db.await;
    let series = RootFolder { kind: RootKind::Series, path: "/media/tv".into() };
    let movies = RootFolder { kind: RootKind::Movies, path: "/media/movies".into() };
    db.add_root_folder(&series).await.unwrap();
    db.add_root_folder(&movies).await.unwrap();

    assert_eq!(db.root_folders().await.unwrap(), [movies.clone(), series]);
    assert!(db.add_root_folder(&movies).await.is_err());
    assert!(db.remove_root_folder(Path::new("/media/tv")).await.unwrap());
    assert!(!db.remove_root_folder(Path::new("/media/tv")).await.unwrap());
    assert_eq!(db.root_folders().await.unwrap(), [movies]);
}

#[rstest]
#[tokio::test]
async fn commit_adds_and_removes_files_with_their_events(#[future] db: Database) {
    let db = db.await;
    let (kept, gone) =
        (file("/tv/a.mkv", episodes(1, 2)), file("/movies/b.mkv", FileTarget::Movie(MovieId(Uuid::from_u128(3)))));
    MediaRepo::save(&db, &Changes { added_files: vec![kept.clone(), gone.clone()], ..Changes::default() }, &[])
        .await
        .unwrap();
    let event = Event::FileDeleted {
        file: gone.id,
        path: gone.path.clone(),
        target: gone.target,
        reason: DeleteReason::External,
    };

    MediaRepo::save(&db, &Changes { removed_files: vec![gone.id], ..Changes::default() }, std::slice::from_ref(&event))
        .await
        .unwrap();

    assert_eq!(db.files().await.unwrap(), [kept]);
    let recorded = db.event_log().read_after(None, 10).await.unwrap();
    assert_eq!(recorded.into_iter().map(|recorded| recorded.event).collect::<Vec<_>>(), [event]);
}

#[rstest]
#[tokio::test]
async fn a_failed_commit_changes_nothing(#[future] db: Database) {
    let db = db.await;
    let existing = file("/tv/a.mkv", episodes(1, 1));
    MediaRepo::save(&db, &Changes { added_files: vec![existing.clone()], ..Changes::default() }, &[]).await.unwrap();
    let pending = import("/tv/b", now(), vec![row("/tv/b/1.mkv", None)]);
    let duplicate = file("/tv/a.mkv", episodes(2, 2));

    let changes = Changes { added_files: vec![duplicate], imports: vec![pending.clone()], ..Changes::default() };
    let event = Event::ImportNeedsReview { import: pending.id, source: pending.source.clone() };
    assert!(MediaRepo::save(&db, &changes, &[event]).await.is_err());

    assert_eq!(db.files().await.unwrap(), [existing]);
    assert_eq!(db.import(pending.id).await.unwrap(), None);
    assert!(db.event_log().read_after(None, 10).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn imports_are_listed_by_status_oldest_first(#[future] db: Database) {
    let db = db.await;
    let newer = import("/tv/newer", now() + 1.hour(), vec![row("/tv/newer/1.mkv", None)]);
    let older = import("/tv/older", now(), vec![row("/tv/older/1.mkv", Some(episodes(1, 1)))]);
    let mut done = import("/tv/done", now(), vec![]);
    done.status = ImportStatus::Done;
    let changes = Changes { imports: vec![newer.clone(), older.clone(), done.clone()], ..Changes::default() };
    MediaRepo::save(&db, &changes, &[]).await.unwrap();

    assert_eq!(db.imports(ImportStatus::NeedsReview).await.unwrap(), [older, newer]);
    assert_eq!(db.imports(ImportStatus::Done).await.unwrap(), [done]);
}

#[rstest]
#[tokio::test]
async fn saving_an_import_again_replaces_its_rows(#[future] db: Database) {
    let db = db.await;
    let mut pending = import("/tv/b", now(), vec![row("/tv/b/1.mkv", None), row("/tv/b/2.mkv", None)]);
    MediaRepo::save(&db, &Changes { imports: vec![pending.clone()], ..Changes::default() }, &[]).await.unwrap();

    pending.rows.truncate(1);
    pending.rows[0].target = Some(episodes(4, 4));
    pending.rows[0].skipped = true;
    pending.status = ImportStatus::Done;
    MediaRepo::save(&db, &Changes { imports: vec![pending.clone()], ..Changes::default() }, &[]).await.unwrap();

    assert_eq!(db.import(pending.id).await.unwrap(), Some(pending));
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

fn any_target() -> impl Strategy<Value = FileTarget> {
    prop_oneof![
        (any::<u128>(), any::<u16>(), any::<u16>(), 0..5u16).prop_map(|(series, season, first, length)| {
            let span = EpisodeSpan::new(season, first, first.saturating_add(length)).unwrap();
            FileTarget::Episodes { series: SeriesId(Uuid::from_u128(series)), span }
        }),
        any::<u128>().prop_map(|movie| FileTarget::Movie(MovieId(Uuid::from_u128(movie)))),
    ]
}

fn any_row() -> impl Strategy<Value = ImportRow> {
    let confidence = prop_oneof![Just(Confidence::Unknown), Just(Confidence::Guess), Just(Confidence::Certain)];
    ("\\PC{1,40}", 0..=i64::MAX as u64, proptest::option::of(any_target()), confidence, any::<bool>()).prop_map(
        |(path, size, target, confidence, skipped)| ImportRow {
            path: PathBuf::from(path),
            size,
            target,
            confidence,
            skipped,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn stored_imports_and_files_read_back_unchanged(
        rows in prop::collection::vec(any_row(), 0..6),
        target in any_target(),
        size in 0..=i64::MAX as u64,
    ) {
        let pending = import("/tv/b", now(), rows);
        let linked = MediaFile { size, ..file("/tv/a.mkv", target) };

        let (stored_import, stored_files) = block_on(async {
            let db = Database::open_in_memory().await.unwrap();
            let changes = Changes { added_files: vec![linked.clone()], imports: vec![pending.clone()], ..Changes::default() };
            MediaRepo::save(&db, &changes, &[]).await.unwrap();
            (db.import(pending.id).await.unwrap(), db.files().await.unwrap())
        });

        prop_assert_eq!(stored_import, Some(pending));
        prop_assert_eq!(stored_files, vec![linked]);
    }
}

#[rstest]
#[tokio::test]
async fn the_catalog_reads_the_library(#[future] db: Database) {
    let db = db.await;
    let mut series = Series::add(
        SeriesMetadata {
            source: ExternalId::Tmdb(1),
            title: "Frieren".into(),
            original_title: "Sousou no Frieren".into(),
            year: Some(2023),
            poster_path: None,
            status: SourceStatus::Returning,
            seasons: vec![],
        },
        MonitorPreset::All,
        now().to_zoned(TimeZone::UTC).date(),
        now(),
    );
    let mut movie = Movie::add(
        MovieMetadata {
            source: ExternalId::Tmdb(2),
            title: "Dune".into(),
            original_title: "Dune".into(),
            year: Some(2021),
            poster_path: None,
            releases: Releases::default(),
        },
        true,
        now(),
    );
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();
    MovieRepo::save(&db, &mut movie, &[]).await.unwrap();

    assert_eq!(db.all_series().await.unwrap(), std::slice::from_ref(&series));
    assert_eq!(db.all_movies().await.unwrap(), std::slice::from_ref(&movie));
    assert_eq!(Catalog::series(&db, series.id).await.unwrap(), Some(series));
    assert_eq!(Catalog::movie(&db, movie.id).await.unwrap(), Some(movie));
    assert_eq!(Catalog::movie(&db, MovieId::generate()).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn renamed_files_keep_their_id_and_target(#[future] db: Database) {
    let db = db.await;
    let moved = file("/tv/a.mkv", episodes(1, 1));
    MediaRepo::save(&db, &Changes { added_files: vec![moved.clone()], ..Changes::default() }, &[]).await.unwrap();

    let changes = Changes { renamed_files: vec![(moved.id, "/tv/Frieren/a.mkv".into())], ..Changes::default() };
    MediaRepo::save(&db, &changes, &[]).await.unwrap();

    assert_eq!(db.files().await.unwrap(), [MediaFile { path: "/tv/Frieren/a.mkv".into(), ..moved }]);
}
