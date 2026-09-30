use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use rstest::rstest;
use tempfile::TempDir;
use yokoku_core::{
    downloads::{
        Download, DownloadError, DownloadOptions, DownloadState, Downloads, TorrentStatus,
        ports::{AddedTorrent, ClientError, DownloadClient, LABEL, Torrent, TorrentSource},
    },
    events::{EventLog, Handler, QueueChanges},
};
use yokoku_domain::{
    Clock, DownloadId, ImportId, ItemId, Live, MovieId, SeriesId,
    events::{DownloadCompleted, Event, FilesImported, TorrentAdded, TorrentRemoved},
};
use yokoku_infra::db::Database;
use yokoku_test_support::{clock::TestClock, events::publisher};

const HASH: &str = "c9e15763f722f23e98a29decdfae341b98d53056";

/// Holds torrents by hash, as a download client would.
#[derive(Default)]
struct ScriptedClient {
    torrents: Mutex<HashMap<String, Torrent>>,
    unavailable: Mutex<bool>,
    /// `(hash, delete_data)` of each removal.
    removed: Mutex<Vec<(String, bool)>>,
}

impl ScriptedClient {
    fn set(&self, done: u64, size: u64) {
        let torrent = torrent(done, size);
        self.torrents.lock().unwrap().insert(torrent.hash.clone(), torrent);
    }

    fn forget(&self) {
        self.torrents.lock().unwrap().clear();
    }

    fn put(&self, torrent: Torrent) {
        self.torrents.lock().unwrap().insert(torrent.hash.clone(), torrent);
    }

    fn finish_seeding(&self) {
        self.torrents.lock().unwrap().values_mut().for_each(|torrent| torrent.seeding_done = true);
    }

    fn check(&self) -> Result<(), ClientError> {
        if *self.unavailable.lock().unwrap() {
            Err(ClientError::Unavailable("connection refused".into()))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl DownloadClient for ScriptedClient {
    async fn version(&self) -> Result<String, ClientError> {
        self.check()?;
        Ok("Transmission 4.1.3".into())
    }

    async fn add(&self, _torrent: &TorrentSource) -> Result<AddedTorrent, ClientError> {
        self.check()?;
        let mut torrents = self.torrents.lock().unwrap();
        let torrent = torrents.entry(HASH.into()).or_insert_with(|| torrent(0, 0));
        Ok(AddedTorrent { hash: torrent.hash.clone(), name: torrent.name.clone() })
    }

    async fn torrents(&self, hashes: &[String]) -> Result<Vec<Torrent>, ClientError> {
        self.check()?;
        let torrents = self.torrents.lock().unwrap();
        Ok(hashes.iter().filter_map(|hash| torrents.get(hash).cloned()).collect())
    }

    async fn all_torrents(&self) -> Result<Vec<Torrent>, ClientError> {
        self.check()?;
        Ok(self.torrents.lock().unwrap().values().cloned().collect())
    }

    async fn remove(&self, hash: &str, delete_data: bool) -> Result<(), ClientError> {
        self.check()?;
        self.torrents.lock().unwrap().remove(hash);
        self.removed.lock().unwrap().push((hash.to_owned(), delete_data));
        Ok(())
    }
}

fn torrent(done: u64, size: u64) -> Torrent {
    let complete = size > 0 && done == size;
    Torrent {
        hash: HASH.into(),
        name: "Dune.2021.1080p".into(),
        status: TorrentStatus {
            state: if complete { DownloadState::Seeding } else { DownloadState::Downloading },
            size,
            done,
            download_rate: if complete { 0 } else { 2_000_000 },
            eta: (!complete).then_some(60),
            download_dir: "/downloads".into(),
            error: None,
        },
        complete,
        seeding_done: false,
        labels: Vec::new(),
    }
}

struct Setup {
    _dir: TempDir,
    db: Database,
    client: Arc<ScriptedClient>,
    changes: QueueChanges,
    downloads: Downloads,
}

async fn setup() -> Setup {
    setup_with(DownloadOptions::default()).await
}

async fn setup_with(options: DownloadOptions) -> Setup {
    let dir = TempDir::new().unwrap();
    let db = Database::open_in_memory().await.unwrap();
    let client = Arc::new(ScriptedClient::default());
    let changes = QueueChanges::new();
    let downloads = Downloads::new(
        Arc::new(db.clone()),
        client.clone(),
        Arc::new(TestClock::default()),
        Live::fixed(options),
        publisher(&db),
        changes.clone(),
    );
    Setup { _dir: dir, db, client, changes, downloads }
}

impl Setup {
    async fn events(&self) -> Vec<Event> {
        let recorded = EventLog::new(self.db.clone()).read_after(None, 100).await.unwrap();
        recorded.into_iter().map(|recorded| recorded.event).collect()
    }

    async fn only_download(&self) -> Download {
        let mut list = self.downloads.list().await.unwrap();
        assert_eq!(list.len(), 1);
        list.pop().unwrap()
    }
}

fn magnet() -> TorrentSource {
    TorrentSource::Magnet(format!("magnet:?xt=urn:btih:{HASH}"))
}

fn completed(download: &Download) -> Event {
    DownloadCompleted {
        download: download.id,
        name: "Dune.2021.1080p".into(),
        content_path: "/downloads/Dune.2021.1080p".into(),
        item: download.item,
        season: download.season,
    }
    .into()
}

#[tokio::test]
async fn adding_records_the_torrent_with_its_item_and_status() {
    let setup = setup().await;
    setup.client.set(250, 1000);
    let item = Some(ItemId::Movie(MovieId::generate()));

    let added = setup.downloads.add(&magnet(), item, None).await.unwrap();

    assert_eq!(setup.only_download().await, added);
    assert_eq!((added.hash.as_str(), added.item, added.percent_done()), (HASH, item, 25));
    assert_eq!(added.status.state, DownloadState::Downloading);
    assert_eq!(setup.events().await, [TorrentAdded { download: added.id, name: added.name.clone(), item }.into()]);
}

#[tokio::test]
async fn a_torrent_is_added_only_once() {
    let setup = setup().await;
    setup.downloads.add(&magnet(), None, None).await.unwrap();

    let error = setup.downloads.add(&magnet(), None, None).await.unwrap_err();

    assert!(matches!(error, DownloadError::AlreadyAdded(_)), "{error}");
    assert_eq!(setup.downloads.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_download_completes_once_however_often_it_syncs() {
    let setup = setup().await;
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.client.set(500, 1000);
    let halfway = setup.downloads.sync().await.unwrap();
    setup.client.set(1000, 1000);

    let finished = setup.downloads.sync().await.unwrap();
    let again = setup.downloads.sync().await.unwrap();

    assert!(halfway.completed.is_empty());
    assert_eq!(finished.completed, [added.id]);
    assert!(again.completed.is_empty());
    let download = setup.only_download().await;
    assert_eq!(download.status.state, DownloadState::Seeding);
    assert_eq!(download.completed_at, Some(TestClock::default().now().timestamp()));
    let events = setup.events().await;
    assert_eq!(events.iter().filter_map(Event::get::<DownloadCompleted>).count(), 1);
    assert_eq!(events.last(), Some(&completed(&download)));
}

#[tokio::test]
async fn a_torrent_already_complete_when_added_completes_at_once() {
    let setup = setup().await;
    setup.client.set(1000, 1000);

    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();

    assert_eq!(setup.events().await[1..], [completed(&added)]);
}

#[tokio::test]
async fn the_season_given_for_a_series_reaches_its_completion() {
    let setup = setup().await;
    setup.client.set(1000, 1000);
    let item = Some(ItemId::Series(SeriesId::generate()));

    let added = setup.downloads.add(&magnet(), item, Some(2)).await.unwrap();

    assert_eq!((setup.only_download().await.season, added.season), (Some(2), Some(2)));
    assert_eq!(setup.events().await[1..], [completed(&added)]);
}

#[tokio::test]
async fn torrents_gone_from_the_client_are_marked_removed_and_left_alone() {
    let setup = setup().await;
    setup.client.set(500, 1000);
    setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.client.forget();

    let report = setup.downloads.sync().await.unwrap();
    setup.client.set(1000, 1000);
    let later = setup.downloads.sync().await.unwrap();

    assert_eq!((report.synced, report.removed), (1, 1));
    assert_eq!(later.synced, 0);
    let download = setup.only_download().await;
    assert_eq!(download.status.state, DownloadState::Removed);
    assert_eq!((download.status.done, download.status.download_rate, download.status.eta), (500, 0, None));
}

#[tokio::test]
async fn an_unreachable_client_changes_nothing() {
    let setup = setup().await;
    setup.client.set(500, 1000);
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();
    *setup.client.unavailable.lock().unwrap() = true;

    let sync = setup.downloads.sync().await.unwrap_err();

    assert!(matches!(sync, DownloadError::Client(ClientError::Unavailable(_))), "{sync}");
    assert_eq!(setup.only_download().await, added);
}

#[tokio::test]
async fn concurrent_syncs_complete_a_download_once() {
    let setup = setup().await;
    setup.client.set(500, 1000);
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.client.set(1000, 1000);

    let (first, second) = tokio::join!(setup.downloads.sync(), setup.downloads.sync());

    let completed = [first.unwrap().completed, second.unwrap().completed].concat();
    assert_eq!(completed, [added.id]);
    let events = setup.events().await;
    assert_eq!(events.iter().filter_map(Event::get::<DownloadCompleted>).count(), 1);
}

fn imported(download: Option<DownloadId>) -> FilesImported {
    FilesImported { import: ImportId::generate(), download, files: Vec::new() }
}

#[tokio::test]
async fn an_import_from_a_download_marks_it_imported_once() {
    let setup = setup().await;
    setup.client.set(1000, 1000);
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();

    setup.downloads.handle(&imported(Some(added.id))).await.unwrap();
    let first = setup.only_download().await;
    setup.downloads.handle(&imported(Some(added.id))).await.unwrap();

    assert_eq!(first.imported_at, Some(TestClock::default().now().timestamp()));
    assert_eq!(setup.only_download().await.revision, first.revision);
}

#[tokio::test]
async fn imports_of_scanned_files_or_unknown_downloads_change_nothing() {
    let setup = setup().await;
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();

    setup.downloads.handle(&imported(None)).await.unwrap();
    setup.downloads.handle(&imported(Some(DownloadId::generate()))).await.unwrap();

    assert_eq!(setup.only_download().await, added);
}

const CLEAN_UP: DownloadOptions =
    DownloadOptions { remove_after_seeding: true, pick_up_labels: Vec::new(), pick_up_folder: None };

#[tokio::test]
async fn an_imported_download_is_removed_with_its_data_once_seeded() {
    let setup = setup_with(CLEAN_UP).await;
    setup.client.set(1000, 1000);
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.downloads.mark_imported(added.id).await.unwrap();
    setup.client.finish_seeding();

    let report = setup.downloads.sync().await.unwrap();
    let again = setup.downloads.sync().await.unwrap();

    assert_eq!((report.cleaned_up, again.synced), (1, 0));
    assert_eq!(*setup.client.removed.lock().unwrap(), [(HASH.to_owned(), true)]);
    assert_eq!(setup.only_download().await.status.state, DownloadState::Removed);
    assert_eq!(
        setup.events().await.last(),
        Some(&TorrentRemoved { download: added.id, name: added.name, item: None }.into())
    );
}

#[rstest]
#[case::not_imported(CLEAN_UP, false, true)]
#[case::still_seeding(CLEAN_UP, true, false)]
#[case::switched_off(DownloadOptions::default(), true, true)]
#[tokio::test]
async fn other_downloads_stay_in_the_client(
    #[case] options: DownloadOptions,
    #[case] imported: bool,
    #[case] seeded: bool,
) {
    let setup = setup_with(options).await;
    setup.client.set(1000, 1000);
    let added = setup.downloads.add(&magnet(), None, None).await.unwrap();
    if imported {
        setup.downloads.mark_imported(added.id).await.unwrap();
    }
    if seeded {
        setup.client.finish_seeding();
    }

    let report = setup.downloads.sync().await.unwrap();

    assert_eq!(report.cleaned_up, 0);
    assert!(setup.client.removed.lock().unwrap().is_empty());
    assert_eq!(setup.only_download().await.status.state, DownloadState::Seeding);
}

/// A finished torrent added outside Yokoku.
fn outside(hash: &str, labels: &[&str], folder: &str) -> Torrent {
    let mut torrent = torrent(1000, 1000);
    torrent.hash = hash.into();
    torrent.name = format!("Show {hash}");
    torrent.labels = labels.iter().map(|label| (*label).to_owned()).collect();
    torrent.status.download_dir = folder.into();
    torrent
}

fn picking_up(labels: &[&str], folder: Option<&str>) -> DownloadOptions {
    DownloadOptions {
        pick_up_labels: labels.iter().map(|label| (*label).to_owned()).collect(),
        pick_up_folder: folder.map(Into::into),
        ..DownloadOptions::default()
    }
}

#[rstest]
#[case::by_label(picking_up(&["tv"], None))]
#[case::by_folder(picking_up(&[], Some("/downloads/tv")))]
#[tokio::test]
async fn qualifying_torrents_from_outside_are_taken_on_once(#[case] options: DownloadOptions) {
    let setup = setup_with(options).await;
    setup.client.put(outside("aa", &["tv"], "/downloads/tv/shows"));
    setup.client.put(outside("bb", &["music"], "/downloads/music"));

    let first = setup.downloads.sync().await.unwrap();
    let second = setup.downloads.sync().await.unwrap();

    assert_eq!((first.picked_up, second.picked_up, second.synced), (1, 0, 1));
    let download = setup.only_download().await;
    assert_eq!((download.hash.as_str(), download.item), ("aa", None));
    assert_eq!(first.completed, [download.id]);
    assert_eq!(
        setup.events().await,
        [
            TorrentAdded { download: download.id, name: "Show aa".into(), item: None }.into(),
            DownloadCompleted {
                download: download.id,
                name: "Show aa".into(),
                content_path: "/downloads/tv/shows/Show aa".into(),
                item: None,
                season: None,
            }
            .into(),
        ]
    );
}

#[tokio::test]
async fn concurrent_syncs_take_on_an_outside_torrent_once() {
    let setup = setup_with(picking_up(&["tv"], None)).await;
    setup.client.put(outside("aa", &["tv"], "/downloads/tv/shows"));

    let (first, second) = tokio::join!(setup.downloads.sync(), setup.downloads.sync());

    assert_eq!(first.unwrap().picked_up + second.unwrap().picked_up, 1);
    assert_eq!(setup.downloads.list().await.unwrap().len(), 1);
    assert_eq!(setup.events().await.iter().filter_map(Event::get::<TorrentAdded>).count(), 1);
}

#[tokio::test]
async fn torrents_from_outside_are_left_alone_unless_asked_for() {
    let setup = setup().await;
    setup.client.put(outside("aa", &["tv"], "/downloads/tv"));

    let report = setup.downloads.sync().await.unwrap();

    assert_eq!(report.picked_up, 0);
    assert!(setup.downloads.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_download_that_was_removed_is_not_taken_on_again() {
    let setup = setup_with(picking_up(&["yokoku"], None)).await;
    setup.client.set(1000, 1000);
    setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.client.forget();
    setup.downloads.sync().await.unwrap();
    let mut back = torrent(1000, 1000);
    back.labels = vec!["yokoku".into()];
    setup.client.put(back);

    let report = setup.downloads.sync().await.unwrap();

    assert_eq!(report.picked_up, 0);
    assert_eq!(setup.only_download().await.status.state, DownloadState::Removed);
}

#[tokio::test]
async fn a_torrent_yokoku_added_but_never_saved_is_taken_on_without_asking() {
    let setup = setup().await;
    setup.client.put(outside("aa", &[LABEL], "/downloads"));

    let report = setup.downloads.sync().await.unwrap();

    assert_eq!(report.picked_up, 1);
    let download = setup.only_download().await;
    assert_eq!((download.hash.as_str(), download.item), ("aa", None));
}

#[tokio::test]
async fn active_syncs_leave_the_client_alone_while_nothing_downloads() {
    let setup = setup().await;
    setup.client.set(1000, 1000);
    setup.downloads.add(&magnet(), None, None).await.unwrap();
    *setup.client.unavailable.lock().unwrap() = true;
    let watch = setup.changes.watch();

    let report = setup.downloads.sync_active().await.unwrap();

    assert_eq!(report, None);
    assert!(!watch.has_changed().unwrap());
}

#[tokio::test]
async fn active_syncs_sync_while_a_download_is_in_progress() {
    let setup = setup().await;
    setup.client.set(250, 1000);
    setup.downloads.add(&magnet(), None, None).await.unwrap();
    setup.client.set(500, 1000);

    let report = setup.downloads.sync_active().await.unwrap();

    assert_eq!(report.map(|report| report.synced), Some(1));
    assert_eq!(setup.only_download().await.percent_done(), 50);
}

#[tokio::test]
async fn adding_and_syncing_are_announced() {
    let setup = setup().await;
    let mut watch = setup.changes.watch();
    setup.client.set(250, 1000);

    setup.downloads.add(&magnet(), None, None).await.unwrap();
    let after_add = watch.has_changed().unwrap();
    watch.borrow_and_update();
    setup.downloads.sync().await.unwrap();

    assert!(after_add);
    assert!(watch.has_changed().unwrap());
}
