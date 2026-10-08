use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

use jiff::{Timestamp, Zoned, tz::TimeZone};
use tokio::{process::Command, time::sleep};
use yokoku_core::{
    downloads::{
        DownloadOptions, DownloadState, Downloads,
        ports::{DownloadClient, TorrentSource},
    },
    events::{EventLog, QueueChanges},
};
use yokoku_domain::{
    Clock, Live, MediaFileId,
    events::{DownloadCompleted, Event},
};
use yokoku_infra::{
    db::Database,
    download_clients::{TransmissionClient, TransmissionSettings},
};
use yokoku_test_support::{events::publisher, services};

const WAIT: Duration = Duration::from_secs(20);

struct SystemTime;

impl Clock for SystemTime {
    fn now(&self) -> Zoned {
        Timestamp::now().to_zoned(TimeZone::UTC)
    }
}

/// A file in Transmission's download folder, removed on drop.
struct Downloaded(PathBuf);

impl Drop for Downloaded {
    fn drop(&mut self) {
        _ = fs::remove_file(&self.0);
    }
}

async fn run(program: &str, args: &[&str]) {
    let status = Command::new(program).args(args).stdout(Stdio::null()).status().await.unwrap();
    assert!(status.success(), "{program} failed");
}

async fn wait_for<T>(mut attempt: impl AsyncFnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = attempt().await {
            return value;
        }
        assert!(start.elapsed() < WAIT, "timed out");
        sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
#[ignore = "needs the services from `just services`"]
async fn a_torrent_of_local_data_is_added_and_completes() {
    let id = MediaFileId::generate().0.simple().to_string();
    let name = format!("Dune.2021.1080p.{}.mkv", &id[id.len() - 8..]);
    let video = Downloaded(services::transmission_downloads().join(&name));
    fs::write(&video.0, (0..2_000_000u32).map(|n| (n % 251) as u8).collect::<Vec<_>>()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let torrent_file = dir.path().join("dune.torrent");
    run("transmission-create", &["-o", path(&torrent_file), path(&video.0)]).await;

    let url = services::transmission_url();
    let client = Arc::new(TransmissionClient::new(Live::fixed(TransmissionSettings {
        url: url.clone(),
        ..TransmissionSettings::default()
    })));
    let version = client.version().await.unwrap();
    assert!(version.starts_with("Transmission 4"), "{version}");

    let db = Database::open_in_memory().await.unwrap();
    let use_case = Downloads::new(
        Arc::new(db.clone()),
        Arc::new(db.clone()),
        client.clone(),
        Arc::new(SystemTime),
        Live::fixed(DownloadOptions::default()),
        publisher(&db),
        QueueChanges::new(),
    );
    let added = use_case.add(&TorrentSource::File(fs::read(&torrent_file).unwrap()), None, None).await.unwrap();
    assert_eq!(added.name, name);

    let completed = wait_for(async || {
        use_case.sync().await.unwrap();
        let download = use_case.list().await.unwrap().pop()?;
        download.completed_at.is_some().then_some(download)
    })
    .await;

    assert_eq!(completed.status.state, DownloadState::Seeding);
    assert_eq!(completed.percent_done(), 100);
    assert_eq!(completed.content_path(), video.0);
    let events: Vec<Event> = EventLog::new(db.clone())
        .read_after(None, 10)
        .await
        .unwrap()
        .into_iter()
        .map(|recorded| recorded.event)
        .collect();
    assert_eq!(
        events.last(),
        Some(
            &DownloadCompleted {
                download: added.id,
                name: added.name,
                content_path: video.0.clone(),
                item: None,
                season: None
            }
            .into()
        )
    );

    let ours = async || client.all_torrents().await.unwrap().into_iter().find(|torrent| torrent.hash == completed.hash);
    let torrent = ours().await.unwrap();
    assert_eq!(torrent.labels, ["yokoku"]);
    assert!(!torrent.seeding_done);

    run("transmission-remote", &[&url, "-t", &completed.hash, "-sr", "0"]).await;
    wait_for(async || ours().await?.seeding_done.then_some(())).await;

    client.remove(&completed.hash, true).await.unwrap();
    wait_for(async || (ours().await.is_none() && !video.0.exists()).then_some(())).await;
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
