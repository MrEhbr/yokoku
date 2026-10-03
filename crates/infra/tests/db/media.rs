use std::path::{Path, PathBuf};

use jiff::{Timestamp, ToSpan};
use proptest::prelude::*;
use rstest::rstest;
use support::{SIZE_BEYOND_U32, block_on, db, now};
use uuid::Uuid;
use yokoku_core::{
    library::ports::MediaFiles,
    media::{
        Episodes, Import, ImportRow, ImportStatus, MediaFile, Resolution, RootFolder, RootKind, RowMatch,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{Confidence, DownloadId, EpisodeSpan, FileTarget, ImportId, MediaFileId, MovieId, SeriesId};
use yokoku_infra::db::Database;

use crate::support;

fn episodes(first: u16, last: u16) -> FileTarget {
    FileTarget::Episodes { series: SeriesId(Uuid::from_u128(1)), span: EpisodeSpan::new(1, first, last).unwrap() }
}

fn file(path: &str, target: FileTarget) -> MediaFile {
    MediaFile { id: MediaFileId::generate(), path: path.into(), size: SIZE_BEYOND_U32, target, added_at: now() }
}

fn import(source: &str, created_at: Timestamp, rows: Vec<ImportRow>) -> Import {
    Import {
        id: ImportId::generate(),
        source: source.into(),
        download: None,
        status: ImportStatus::NeedsReview,
        error: None,
        rows,
        created_at,
    }
}

fn row(path: &str, target: Option<FileTarget>) -> ImportRow {
    ImportRow {
        path: path.into(),
        size: 7,
        matched: target.map_or(RowMatch::None, RowMatch::from),
        confidence: Confidence::Guess,
        skipped: false,
        resolution: Resolution::Unresolved,
    }
}

#[rstest]
#[tokio::test]
async fn root_folders_are_listed_by_path_with_their_names_and_removed_by_path(#[future(awt)] db: Database) {
    let series = RootFolder::new(RootKind::Series, "/media/tv".into(), Some("Shows".into()), false);
    let movies = RootFolder::new(RootKind::Movies, "/media/movies".into(), None, false);
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
async fn commit_adds_and_removes_files(#[future(awt)] db: Database) {
    let (kept, gone) =
        (file("/tv/a.mkv", episodes(1, 2)), file("/movies/b.mkv", FileTarget::Movie(MovieId(Uuid::from_u128(3)))));
    MediaRepo::save(&db, &Changes { added_files: vec![kept.clone(), gone.clone()], ..Changes::default() })
        .await
        .unwrap();

    MediaRepo::save(&db, &Changes { removed_files: vec![gone.id], ..Changes::default() }).await.unwrap();

    assert_eq!(db.files().await.unwrap(), [kept]);
}

#[rstest]
#[tokio::test]
async fn a_failed_commit_changes_nothing(#[future(awt)] db: Database) {
    let existing = file("/tv/a.mkv", episodes(1, 1));
    MediaRepo::save(&db, &Changes { added_files: vec![existing.clone()], ..Changes::default() }).await.unwrap();
    let pending = import("/tv/b", now(), vec![row("/tv/b/1.mkv", None)]);
    let duplicate = file("/tv/a.mkv", episodes(2, 2));

    let changes = Changes { added_files: vec![duplicate], imports: vec![pending.clone()], ..Changes::default() };
    assert!(MediaRepo::save(&db, &changes).await.is_err());

    assert_eq!(db.files().await.unwrap(), [existing]);
    assert_eq!(db.import(pending.id).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn imports_are_listed_by_status_oldest_first(#[future(awt)] db: Database) {
    let newer = import("/tv/newer", now() + 1.hour(), vec![row("/tv/newer/1.mkv", None)]);
    let older = import("/tv/older", now(), vec![row("/tv/older/1.mkv", Some(episodes(1, 1)))]);
    let mut done = import("/tv/done", now(), vec![]);
    done.status = ImportStatus::Done;
    let changes = Changes { imports: vec![newer.clone(), older.clone(), done.clone()], ..Changes::default() };
    MediaRepo::save(&db, &changes).await.unwrap();

    assert_eq!(db.imports(ImportStatus::NeedsReview).await.unwrap(), [older, newer]);
    assert_eq!(db.imports(ImportStatus::Done).await.unwrap(), [done]);
}

#[rstest]
#[tokio::test]
async fn saving_an_import_again_replaces_its_rows(#[future(awt)] db: Database) {
    let mut pending = import("/tv/b", now(), vec![row("/tv/b/1.mkv", None), row("/tv/b/2.mkv", None)]);
    MediaRepo::save(&db, &Changes { imports: vec![pending.clone()], ..Changes::default() }).await.unwrap();

    pending.rows.truncate(1);
    pending.rows[0].matched = episodes(4, 4).into();
    pending.rows[0].skipped = true;
    pending.status = ImportStatus::Done;
    MediaRepo::save(&db, &Changes { imports: vec![pending.clone()], ..Changes::default() }).await.unwrap();

    assert_eq!(db.import(pending.id).await.unwrap(), Some(pending));
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

/// Complete matches, and series rows missing their season, their episodes, or both.
fn any_row_match() -> impl Strategy<Value = RowMatch> {
    let episodes =
        (any::<u16>(), 0..5u16).prop_map(|(first, length)| Episodes { first, last: first.saturating_add(length) });
    prop_oneof![
        Just(RowMatch::None),
        any_target().prop_map(RowMatch::from),
        (any::<u128>(), proptest::option::of(any::<u16>()), proptest::option::of(episodes)).prop_map(
            |(series, season, episodes)| RowMatch::Series {
                series: SeriesId(Uuid::from_u128(series)),
                season,
                episodes
            }
        ),
    ]
}

fn any_status() -> impl Strategy<Value = ImportStatus> {
    prop_oneof![
        Just(ImportStatus::NeedsReview),
        Just(ImportStatus::Approved),
        Just(ImportStatus::Importing),
        Just(ImportStatus::Done),
        Just(ImportStatus::Failed),
    ]
}

fn any_row() -> impl Strategy<Value = ImportRow> {
    let confidence = prop_oneof![Just(Confidence::Unknown), Just(Confidence::Guess), Just(Confidence::Certain)];
    let resolution = prop_oneof![Just(Resolution::Unresolved), Just(Resolution::Replace)];
    ("\\PC{1,40}", 0..=i64::MAX as u64, any_row_match(), confidence, any::<bool>(), resolution).prop_map(
        |(path, size, matched, confidence, skipped, resolution)| ImportRow {
            path: PathBuf::from(path),
            size,
            matched,
            confidence,
            skipped,
            resolution,
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
        download in proptest::option::of(any::<u128>()),
        status in any_status(),
        error in proptest::option::of("\\PC{0,40}"),
    ) {
        let pending = Import {
            download: download.map(|id| DownloadId(Uuid::from_u128(id))),
            status,
            error,
            ..import("/tv/b", now(), rows)
        };
        let linked = MediaFile { size, ..file("/tv/a.mkv", target) };

        let (stored_import, stored_files) = block_on(async {
            let db = Database::open_in_memory().await.unwrap();
            let changes = Changes { added_files: vec![linked.clone()], imports: vec![pending.clone()], ..Changes::default() };
            MediaRepo::save(&db, &changes).await.unwrap();
            (db.import(pending.id).await.unwrap(), db.files().await.unwrap())
        });

        prop_assert_eq!(stored_import, Some(pending));
        prop_assert_eq!(stored_files, vec![linked]);
    }
}

#[rstest]
#[tokio::test]
async fn renamed_files_keep_their_id_and_target(#[future(awt)] db: Database) {
    let moved = file("/tv/a.mkv", episodes(1, 1));
    MediaRepo::save(&db, &Changes { added_files: vec![moved.clone()], ..Changes::default() }).await.unwrap();

    let changes = Changes { renamed_files: vec![(moved.id, "/tv/Frieren/a.mkv".into())], ..Changes::default() };
    MediaRepo::save(&db, &changes).await.unwrap();

    assert_eq!(db.files().await.unwrap(), [MediaFile { path: "/tv/Frieren/a.mkv".into(), ..moved }]);
}

#[rstest]
#[tokio::test]
async fn claimed_paths_are_rows_of_unfinished_imports_and_skipped_rows(#[future(awt)] db: Database) {
    let mut imports = Vec::new();
    for (status, name) in [
        (ImportStatus::NeedsReview, "review"),
        (ImportStatus::Approved, "approved"),
        (ImportStatus::Importing, "importing"),
        (ImportStatus::Failed, "failed"),
        (ImportStatus::Done, "done"),
    ] {
        let skipped = ImportRow { skipped: true, ..row(&format!("/{name}/skipped.mkv"), None) };
        let rows = vec![row(&format!("/{name}/kept.mkv"), None), skipped];
        imports.push(Import { status, ..import(&format!("/{name}"), now(), rows) });
    }
    MediaRepo::save(&db, &Changes { imports, ..Changes::default() }).await.unwrap();

    let mut claimed = db.claimed_paths().await.unwrap();
    claimed.sort();

    let unfinished = ["approved", "failed", "importing", "review"]
        .into_iter()
        .flat_map(|name| [format!("/{name}/kept.mkv"), format!("/{name}/skipped.mkv")]);
    let mut expected: Vec<PathBuf> = unfinished.chain(["/done/skipped.mkv".into()]).map(PathBuf::from).collect();
    expected.sort();
    assert_eq!(claimed, expected);
}

#[rstest]
#[tokio::test]
async fn the_library_reads_where_a_file_is_now(#[future(awt)] db: Database) {
    let linked = file("/tv/a.mkv", episodes(1, 1));
    MediaRepo::save(&db, &Changes { added_files: vec![linked.clone()], ..Changes::default() }).await.unwrap();
    let changes = Changes { retargeted_files: vec![(linked.id, episodes(3, 4))], ..Changes::default() };
    MediaRepo::save(&db, &changes).await.unwrap();

    assert_eq!(MediaFiles::target(&db, linked.id).await.unwrap(), Some(episodes(3, 4)));
    assert_eq!(MediaFiles::target(&db, MediaFileId::generate()).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn approved_imports_are_claimed_oldest_first_and_once(#[future(awt)] db: Database) {
    let approved = |source: &str, created_at: Timestamp| Import {
        status: ImportStatus::Approved,
        ..import(source, created_at, vec![row(&format!("{source}/a.mkv"), None)])
    };
    let (older, newer) = (approved("/downloads/a", now()), approved("/downloads/b", now() + 1.hour()));
    let waiting = import("/downloads/c", now() - 1.hour(), vec![]);
    let changes = Changes { imports: vec![newer.clone(), older.clone(), waiting], ..Changes::default() };
    MediaRepo::save(&db, &changes).await.unwrap();

    let first = db.claim_next_approved().await.unwrap().unwrap();
    let second = db.claim_next_approved().await.unwrap().unwrap();
    let none = db.claim_next_approved().await.unwrap();

    assert_eq!((first.id, first.status, first.rows), (older.id, ImportStatus::Importing, older.rows));
    assert_eq!(second.id, newer.id);
    assert_eq!(none, None);
    assert_eq!(db.reset_importing().await.unwrap(), 2);
    assert_eq!(db.imports(ImportStatus::Approved).await.unwrap().len(), 2);
}

#[rstest]
#[tokio::test]
async fn a_download_has_at_most_one_import(#[future(awt)] db: Database) {
    let download = DownloadId::generate();
    let first = Import { download: Some(download), ..import("/downloads/a", now(), vec![]) };
    let second = Import { download: Some(download), ..import("/downloads/a", now(), vec![]) };
    MediaRepo::save(&db, &Changes { imports: vec![first.clone()], ..Changes::default() }).await.unwrap();

    let duplicate = MediaRepo::save(&db, &Changes { imports: vec![second], ..Changes::default() }).await;

    assert!(duplicate.is_err());
    assert_eq!(db.import_for_download(download).await.unwrap(), Some(first));
    assert_eq!(db.import_for_download(DownloadId::generate()).await.unwrap(), None);
}
