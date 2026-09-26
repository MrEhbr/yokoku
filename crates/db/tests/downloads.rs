use jiff::Timestamp;
use proptest::prelude::*;
use yokoku_db::Database;
use yokoku_domain::{DownloadId, ItemId, MovieId, SeriesId, StorageError};
use yokoku_downloads::{Download, DownloadState, DownloadStatus, ports::DownloadRepo};

fn any_state() -> impl Strategy<Value = DownloadState> {
    prop_oneof![
        Just(DownloadState::Queued),
        Just(DownloadState::Checking),
        Just(DownloadState::Downloading),
        Just(DownloadState::Seeding),
        Just(DownloadState::Stopped),
        Just(DownloadState::Removed),
    ]
}

fn any_item() -> impl Strategy<Value = Option<ItemId>> {
    proptest::option::of(prop_oneof![
        any::<u128>().prop_map(|id| ItemId::Series(SeriesId(uuid::Uuid::from_u128(id)))),
        any::<u128>().prop_map(|id| ItemId::Movie(MovieId(uuid::Uuid::from_u128(id)))),
    ])
}

fn any_download() -> impl Strategy<Value = Download> {
    let size = 0..=i64::MAX as u64;
    let status = (
        any_state(),
        size.clone(),
        size.clone(),
        size,
        proptest::option::of(0..=i64::MAX as u64),
        "\\PC{0,30}",
        proptest::option::of("\\PC{0,30}"),
    )
        .prop_map(|(state, size, done, download_rate, eta, download_dir, error)| DownloadStatus {
            state,
            size,
            done,
            download_rate,
            eta,
            download_dir: download_dir.into(),
            error,
        });
    ("[0-9a-f]{40}", "\\PC{1,30}", any_item(), status, any::<bool>(), any::<bool>()).prop_map(
        |(hash, name, item, status, completed, imported)| {
            let added_at: Timestamp = "2026-09-26T12:00:00.5Z".parse().unwrap();
            Download {
                id: DownloadId::generate(),
                hash,
                name,
                item,
                status,
                added_at,
                completed_at: completed.then_some(added_at),
                imported_at: (completed && imported).then_some(added_at),
                revision: 0,
            }
        },
    )
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn stored_downloads_read_back_unchanged(mut download in any_download()) {
        let (by_id, by_hash, listed) = block_on(async {
            let db = Database::open_in_memory().await.unwrap();
            db.save(&mut download, &[]).await.unwrap();
            (db.get(download.id).await.unwrap(), db.find_by_hash(&download.hash).await.unwrap(), db.list().await.unwrap())
        });

        prop_assert_eq!(by_id, Some(download.clone()));
        prop_assert_eq!(by_hash, Some(download.clone()));
        prop_assert_eq!(listed, vec![download]);
    }
}

fn download(hash: &str, added_at: &str) -> Download {
    Download {
        id: DownloadId::generate(),
        hash: hash.into(),
        name: hash.into(),
        item: None,
        status: DownloadStatus::unknown(),
        added_at: added_at.parse().unwrap(),
        completed_at: None,
        imported_at: None,
        revision: 0,
    }
}

#[tokio::test]
async fn downloads_are_listed_newest_first() {
    let db = Database::open_in_memory().await.unwrap();
    let mut older = download("a", "2026-09-25T12:00:00Z");
    let mut newer = download("b", "2026-09-26T12:00:00Z");
    db.save(&mut older, &[]).await.unwrap();
    db.save(&mut newer, &[]).await.unwrap();

    assert_eq!(db.list().await.unwrap(), [newer, older]);
}

#[tokio::test]
async fn a_second_download_with_a_stored_hash_conflicts() {
    let db = Database::open_in_memory().await.unwrap();
    db.save(&mut download("a", "2026-09-25T12:00:00Z"), &[]).await.unwrap();

    let error = db.save(&mut download("a", "2026-09-26T12:00:00Z"), &[]).await.unwrap_err();

    assert!(matches!(error, StorageError::Conflict), "{error}");
}
