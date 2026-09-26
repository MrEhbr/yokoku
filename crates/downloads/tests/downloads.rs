use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use jiff::{
    Zoned,
    civil::{Date, date},
    tz::TimeZone,
};
use yokoku_db::Database;
use yokoku_domain::{Clock, ItemId, MovieId};
use yokoku_downloads::{
    Download, DownloadError, DownloadState, DownloadStatus, Downloads,
    ports::{AddedTorrent, ClientError, DownloadClient, Torrent, TorrentSource},
};
use yokoku_events::{Event, EventLog};

const TODAY: Date = date(2026, 9, 26);
const HASH: &str = "c9e15763f722f23e98a29decdfae341b98d53056";

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Zoned {
        TODAY.at(12, 0, 0, 0).to_zoned(TimeZone::UTC).unwrap()
    }
}

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

    fn check(&self) -> Result<(), ClientError> {
        match *self.unavailable.lock().unwrap() {
            true => Err(ClientError::Unavailable("connection refused".into())),
            false => Ok(()),
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
        status: DownloadStatus {
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
    db: Database,
    client: Arc<ScriptedClient>,
    downloads: Downloads,
}

async fn setup() -> Setup {
    let db = Database::open_in_memory().await.unwrap();
    let client = Arc::new(ScriptedClient::default());
    let downloads = Downloads::new(Arc::new(db.clone()), client.clone(), Arc::new(FixedClock));
    Setup { db, client, downloads }
}

impl Setup {
    async fn events(&self) -> Vec<Event> {
        let recorded = self.db.event_log().read_after(None, 100).await.unwrap();
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
    Event::DownloadCompleted {
        download: download.id,
        name: "Dune.2021.1080p".into(),
        content_path: "/downloads/Dune.2021.1080p".into(),
        item: download.item,
    }
}

#[tokio::test]
async fn adding_records_the_torrent_with_its_item_and_status() {
    let setup = setup().await;
    setup.client.set(250, 1000);
    let item = Some(ItemId::Movie(MovieId::generate()));

    let added = setup.downloads.add(&magnet(), item).await.unwrap();

    assert_eq!(setup.only_download().await, added);
    assert_eq!((added.hash.as_str(), added.item, added.percent_done()), (HASH, item, 25));
    assert_eq!(added.status.state, DownloadState::Downloading);
    assert_eq!(setup.events().await, [Event::TorrentAdded { download: added.id, name: added.name.clone(), item }]);
}

#[tokio::test]
async fn a_torrent_is_added_only_once() {
    let setup = setup().await;
    setup.downloads.add(&magnet(), None).await.unwrap();

    let error = setup.downloads.add(&magnet(), None).await.unwrap_err();

    assert!(matches!(error, DownloadError::AlreadyAdded(_)), "{error}");
    assert_eq!(setup.downloads.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_download_completes_once_however_often_it_syncs() {
    let setup = setup().await;
    let added = setup.downloads.add(&magnet(), None).await.unwrap();
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
    assert_eq!(download.completed_at, Some(FixedClock.now().timestamp()));
    let events = setup.events().await;
    assert_eq!(events.iter().filter(|event| matches!(event, Event::DownloadCompleted { .. })).count(), 1);
    assert_eq!(events.last(), Some(&completed(&download)));
}

#[tokio::test]
async fn a_torrent_already_complete_when_added_completes_at_once() {
    let setup = setup().await;
    setup.client.set(1000, 1000);

    let added = setup.downloads.add(&magnet(), None).await.unwrap();

    assert_eq!(setup.events().await[1..], [completed(&added)]);
}

#[tokio::test]
async fn torrents_gone_from_the_client_are_marked_removed_and_left_alone() {
    let setup = setup().await;
    setup.client.set(500, 1000);
    setup.downloads.add(&magnet(), None).await.unwrap();
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
    let added = setup.downloads.add(&magnet(), None).await.unwrap();
    *setup.client.unavailable.lock().unwrap() = true;

    let sync = setup.downloads.sync().await.unwrap_err();
    let test = setup.downloads.test_connection().await.unwrap_err();

    assert!(matches!(sync, DownloadError::Client(ClientError::Unavailable(_))), "{sync}");
    assert!(matches!(test, DownloadError::Client(_)), "{test}");
    assert_eq!(setup.only_download().await, added);
}

#[tokio::test]
async fn concurrent_syncs_complete_a_download_once() {
    let setup = setup().await;
    setup.client.set(500, 1000);
    let added = setup.downloads.add(&magnet(), None).await.unwrap();
    setup.client.set(1000, 1000);

    let (first, second) = tokio::join!(setup.downloads.sync(), setup.downloads.sync());

    let completed = [first.unwrap().completed, second.unwrap().completed].concat();
    assert_eq!(completed, [added.id]);
    let events = setup.events().await;
    assert_eq!(events.iter().filter(|event| matches!(event, Event::DownloadCompleted { .. })).count(), 1);
}
