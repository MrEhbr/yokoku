use std::{
    fs,
    net::TcpListener,
    path::Path,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

use jiff::{Timestamp, Zoned, tz::TimeZone};
use tokio::{process::Command, time::sleep};
use yokoku_db::Database;
use yokoku_domain::{Clock, Live};
use yokoku_download_clients::{TransmissionClient, TransmissionSettings};
use yokoku_downloads::{
    DownloadOptions, DownloadState, Downloads,
    ports::{DownloadClient, TorrentSource},
};
use yokoku_events::{DownloadCompleted, Event, EventLog, Publisher, QueueChanges};
use yokoku_system::FileSpool;

const WAIT: Duration = Duration::from_secs(20);

struct SystemTime;

impl Clock for SystemTime {
    fn now(&self) -> Zoned {
        Timestamp::now().to_zoned(TimeZone::UTC)
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
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
#[ignore = "needs transmission-daemon, transmission-create and transmission-remote on PATH"]
async fn a_torrent_of_local_data_is_added_and_completes() {
    let dir = tempfile::tempdir().unwrap();
    let downloads = dir.path().join("downloads");
    fs::create_dir_all(&downloads).unwrap();
    let video = downloads.join("Dune.2021.1080p.mkv");
    fs::write(&video, (0..2_000_000u32).map(|n| (n % 251) as u8).collect::<Vec<_>>()).unwrap();
    let torrent_file = dir.path().join("dune.torrent");
    run("transmission-create", &["-o", path(&torrent_file), path(&video)]).await;

    let port = free_port().to_string();
    let peer_port = free_port().to_string();
    let config = dir.path().join("config");
    let _daemon = Command::new("transmission-daemon")
        .args(["-f", "-g", path(&config), "-p", &port, "-T", "-w", path(&downloads), "-M", "-O", "-Y"])
        .args(["-P", &peer_port, "--log-level=error"])
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    let client = Arc::new(TransmissionClient::new(Live::fixed(TransmissionSettings {
        url: format!("http://127.0.0.1:{port}/transmission/rpc"),
        ..TransmissionSettings::default()
    })));
    let version = wait_for(async || client.version().await.ok()).await;
    assert!(version.starts_with("Transmission 4"), "{version}");

    let db = Database::open_in_memory().await.unwrap();
    let use_case = Downloads::new(
        Arc::new(db.clone()),
        client.clone(),
        Arc::new(SystemTime),
        Live::fixed(DownloadOptions::default()),
        Publisher::new(Arc::new(db.event_log()), Arc::new(FileSpool::new(dir.path().join("yokoku.spool")))),
        QueueChanges::new(),
    );
    let added = use_case.add(&TorrentSource::File(fs::read(&torrent_file).unwrap()), None).await.unwrap();
    assert_eq!(added.name, "Dune.2021.1080p.mkv");

    let completed = wait_for(async || {
        use_case.sync().await.unwrap();
        let download = use_case.list().await.unwrap().pop()?;
        download.completed_at.is_some().then_some(download)
    })
    .await;

    assert_eq!(completed.status.state, DownloadState::Seeding);
    assert_eq!(completed.percent_done(), 100);
    assert_eq!(completed.content_path(), video);
    let events: Vec<Event> =
        db.event_log().read_after(None, 10).await.unwrap().into_iter().map(|recorded| recorded.event).collect();
    assert_eq!(
        events.last(),
        Some(
            &DownloadCompleted { download: added.id, name: added.name, content_path: video.clone(), item: None }.into()
        )
    );

    let all = client.all_torrents().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].labels, ["yokoku"]);
    assert!(!all[0].seeding_done);

    run("transmission-remote", &[&format!("127.0.0.1:{port}"), "-t", "all", "-sr", "0"]).await;
    wait_for(async || client.all_torrents().await.unwrap()[0].seeding_done.then_some(())).await;

    client.remove(&completed.hash, true).await.unwrap();
    wait_for(async || (client.all_torrents().await.unwrap().is_empty() && !video.exists()).then_some(())).await;
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
