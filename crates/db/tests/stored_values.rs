use std::path::PathBuf;

use jiff::{Timestamp, civil::date};
use rstest::rstest;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use yokoku_db::Database;
use yokoku_domain::{
    DownloadId, EpisodeMetadata, ExternalId, MonitorPreset, SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_downloads::{Download, DownloadState, DownloadStatus, ports::DownloadRepo};
use yokoku_library::ports::SeriesRepo;

fn now() -> Timestamp {
    "2026-09-26T12:00:00Z".parse().unwrap()
}

fn series(episode_source_id: u64) -> Series {
    let metadata = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: vec![],
        year: None,
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: vec![SeasonMetadata {
            number: 1,
            episodes: vec![EpisodeMetadata {
                source_id: episode_source_id,
                number: 1,
                title: "Episode 1".into(),
                air_date: None,
            }],
        }],
    };
    Series::add(metadata, MonitorPreset::All, date(2026, 9, 26), now())
}

#[rstest]
#[case::timestamp("UPDATE series SET added_at = 'yesterday'")]
#[case::source_status("UPDATE series SET source_status = 'airing'")]
#[case::numbering("UPDATE series SET numbering = 'Absolute'")]
#[case::negative_source_id("UPDATE series SET source_id = -1")]
#[case::negative_revision("UPDATE series SET revision = -1")]
#[case::alternate_titles("UPDATE series SET alternate_titles = 'not json'")]
#[case::episode_id("UPDATE episodes SET id = 'not-a-uuid'")]
#[case::air_date("UPDATE episodes SET air_date = '2026-13-01'")]
#[case::file_id("UPDATE episodes SET file_id = 'not-a-uuid'")]
#[case::negative_episode_source_id("UPDATE episodes SET source_id = -1")]
#[tokio::test]
async fn loading_a_series_with_a_bad_stored_value_fails(#[case] corruption: &'static str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let db = Database::open(&path).await.unwrap();
    let mut series = series(1001);
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

    let raw = SqlitePool::connect_with(SqliteConnectOptions::new().filename(&path)).await.unwrap();
    sqlx::query(corruption).execute(&raw).await.unwrap();

    assert!(SeriesRepo::get(&db, series.id).await.is_err());
}

#[tokio::test]
async fn saving_an_episode_source_id_beyond_i64_fails() {
    let db = Database::open_in_memory().await.unwrap();
    let mut series = series(u64::MAX);

    assert!(SeriesRepo::save(&db, &mut series, &[]).await.is_err());
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), None);
}

#[tokio::test]
async fn saving_a_download_size_beyond_i64_fails() {
    let db = Database::open_in_memory().await.unwrap();
    let mut download = Download {
        id: DownloadId::generate(),
        hash: "abc".into(),
        name: "Frieren".into(),
        item: None,
        status: DownloadStatus {
            state: DownloadState::Downloading,
            size: u64::MAX,
            done: 0,
            download_rate: 0,
            eta: None,
            download_dir: PathBuf::from("/downloads"),
            error: None,
        },
        added_at: now(),
        completed_at: None,
        imported_at: None,
        revision: 0,
    };

    assert!(DownloadRepo::save(&db, &mut download, &[]).await.is_err());
    assert_eq!(DownloadRepo::get(&db, download.id).await.unwrap(), None);
}

#[cfg(unix)]
#[tokio::test]
async fn adding_a_non_utf8_root_folder_fails() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    use yokoku_media::{RootFolder, RootKind, ports::MediaRepo};

    let db = Database::open_in_memory().await.unwrap();
    let root = RootFolder { kind: RootKind::Series, path: PathBuf::from(OsStr::from_bytes(b"/media/\xff")) };

    assert!(db.add_root_folder(&root).await.is_err());
    assert_eq!(db.root_folders().await.unwrap(), []);
}
