use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use jiff::Timestamp;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{body_partial_json, header, method},
};
use yokoku_db::Database;
use yokoku_domain::{ExternalId, Movie, MovieMetadata, Releases};
use yokoku_events::{Event, EventLog};
use yokoku_library::ports::MovieRepo;

const HASH: &str = "0638ffbb73b3f3ef1ba1fbbfa05a7e1db69610f6";
const SESSION: &str = "session-1";

struct Setup {
    dir: TempDir,
    database: PathBuf,
    transmission: MockServer,
}

impl Setup {
    /// A library with "Dune" (tmdb:10) and a movie root, a finished download of it on disk, and a
    /// Transmission that knows the session handshake.
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("yokoku.db");
        let db = Database::open(&database).await.unwrap();
        let dune = MovieMetadata {
            source: ExternalId::Tmdb(10),
            title: "Dune".into(),
            original_title: "Dune".into(),
            alternate_titles: Vec::new(),
            year: Some(2021),
            poster_path: None,
            releases: Releases::default(),
        };
        MovieRepo::save(&db, &mut Movie::add(dune, true, Timestamp::now()), &[]).await.unwrap();

        let transmission = MockServer::start().await;
        Mock::given(method("POST"))
            .and(|request: &Request| !request.headers.contains_key("X-Transmission-Session-Id"))
            .respond_with(ResponseTemplate::new(409).insert_header("X-Transmission-Session-Id", SESSION))
            .mount(&transmission)
            .await;
        let video = dir.path().join("downloads/Dune.2021.1080p/Dune.2021.1080p.mkv");
        std::fs::create_dir_all(video.parent().unwrap()).unwrap();
        std::fs::write(&video, b"video").unwrap();
        std::fs::create_dir(dir.path().join("movies")).unwrap();
        let setup = Self { dir, database, transmission };
        let movies = setup.dir.path().join("movies");
        setup.stdout(&["root", "add", "movies", movies.to_str().unwrap()]);
        setup
    }

    async fn answer(&self, rpc_method: &str, arguments: Value) {
        Mock::given(header("X-Transmission-Session-Id", SESSION))
            .and(body_partial_json(json!({ "method": rpc_method })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "arguments": arguments, "result": "success" })),
            )
            .mount(&self.transmission)
            .await;
    }

    async fn torrent_at(&self, left: u64) {
        self.transmission.reset().await;
        Mock::given(method("POST"))
            .and(|request: &Request| !request.headers.contains_key("X-Transmission-Session-Id"))
            .respond_with(ResponseTemplate::new(409).insert_header("X-Transmission-Session-Id", SESSION))
            .mount(&self.transmission)
            .await;
        self.answer(
            "torrent-add",
            json!({ "torrent-added": { "hashString": HASH, "id": 1, "name": "Dune.2021.1080p" } }),
        )
        .await;
        let torrent = json!({
            "hashString": HASH, "name": "Dune.2021.1080p", "status": if left == 0 { 6 } else { 4 },
            "sizeWhenDone": 4_000_000_000u64, "leftUntilDone": left, "rateDownload": 5_000_000, "eta": 600,
            "downloadDir": self.dir.path().join("downloads"), "error": 0, "errorString": "", "metadataPercentComplete": 1.0,
            "isFinished": false, "labels": ["yokoku"],
        });
        self.answer("torrent-get", json!({ "torrents": [torrent] })).await;
    }

    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command
            .env("APP__DATABASE__PATH", &self.database)
            .env("APP__CLOCK__TIMEZONE", "UTC")
            .env("APP__TRANSMISSION__URL", format!("{}/transmission/rpc", self.transmission.uri()));
        command
    }

    fn stdout(&self, args: &[&str]) -> String {
        let output = self.command().args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }

    async fn events(&self) -> Vec<Event> {
        let db = Database::open(&self.database).await.unwrap();
        db.event_log().read_after(None, 100).await.unwrap().into_iter().map(|recorded| recorded.event).collect()
    }
}

#[tokio::test]
async fn test_reports_the_transmission_version() {
    let setup = Setup::new().await;
    setup.answer("session-get", json!({ "version": "4.1.3 (0)" })).await;

    assert_eq!(setup.stdout(&["download", "test"]), "Connected to Transmission 4.1.3 (0)\n");
}

#[tokio::test]
async fn an_unreachable_transmission_fails_the_test() {
    let setup = Setup::new().await;

    setup
        .command()
        .args(["download", "test"])
        .env("APP__TRANSMISSION__URL", "http://127.0.0.1:9/transmission/rpc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("download client unavailable"));
}

#[tokio::test]
async fn added_downloads_are_listed_and_finish_on_sync() {
    let setup = Setup::new().await;
    setup.torrent_at(1_000_000_000).await;

    let added = setup.stdout(&["download", "add", &format!("magnet:?xt=urn:btih:{HASH}"), "movie", "tmdb:10"]);
    let listed = setup.stdout(&["download", "list"]);
    setup.torrent_at(0).await;
    let synced = setup.stdout(&["download", "sync"]);

    assert_eq!(added, "Added Dune.2021.1080p\n");
    assert!(listed.starts_with("Dune.2021.1080p"), "{listed}");
    assert!(
        listed.contains(" 75%  downloading  5.0 MB/s, 0h 10m left") && listed.ends_with("Dune (2021)\n"),
        "{listed}"
    );
    let content = setup.dir.path().join("downloads/Dune.2021.1080p");
    assert_eq!(synced, format!("Synced 1 downloads; 1 finished\nImported {}\n", content.display()));
    assert!(setup.stdout(&["download", "list"]).contains("100%  seeding"));
    let placed = setup.dir.path().join("movies/Dune (2021)/Dune (2021).mkv");
    assert_eq!(inode(&placed), inode(&content.join("Dune.2021.1080p.mkv")));
    assert!(setup.stdout(&["show", "movie", "tmdb:10"]).contains("File      downloaded"));
    let events = setup.events().await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DownloadCompleted { content_path, .. } if content_path == &content))
    );
    assert!(matches!(events.last(), Some(Event::FilesImported { .. })));
}

fn inode(path: &std::path::Path) -> u64 {
    std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(path).unwrap())
}

#[tokio::test]
async fn an_empty_list_says_so() {
    let setup = Setup::new().await;

    assert_eq!(setup.stdout(&["download", "list"]), "No downloads.\n");
}

#[test]
fn torrent_files_that_cannot_be_read_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .args(["download", "add", "missing.torrent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read missing.torrent"));
}

#[tokio::test(flavor = "multi_thread")]
async fn serve_syncs_and_imports_downloads_on_schedule_and_stops_on_sigterm() {
    let setup = Setup::new().await;
    setup.torrent_at(1_000_000_000).await;
    setup.stdout(&["download", "add", &format!("magnet:?xt=urn:btih:{HASH}")]);
    setup.torrent_at(0).await;

    let mut serve = setup
        .command()
        .arg("serve")
        .env("APP__SERVE__SYNC_DOWNLOADS", "* * * * * *")
        .env("APP__SERVE__EXECUTE_IMPORTS", "* * * * * *")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let db = Database::open(&setup.database).await.unwrap();
    let log = db.event_log();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let delivered = loop {
        let events = log.read_after(None, 100).await.unwrap();
        let last = events.last().unwrap();
        let delivered = log.last_delivered("library.files").await.unwrap();
        if matches!(last.event, Event::FilesImported { .. }) && delivered == Some(last.id) {
            break true;
        }
        if std::time::Instant::now() > deadline {
            break false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    };
    let killed = Command::new("kill").args(["-TERM", &serve.id().to_string()]).status().unwrap();
    let status = serve.wait().unwrap();

    assert!(delivered, "the download was not synced, imported and delivered in time");
    assert!(killed.success());
    assert!(status.success(), "{status}");
}

#[tokio::test]
async fn a_failed_import_is_listed_and_retried() {
    let setup = Setup::new().await;
    let blocker = setup.dir.path().join("movies/Dune (2021)/Dune (2021).mkv");
    std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    std::fs::write(&blocker, b"another film").unwrap();
    setup.torrent_at(0).await;

    let added = setup.stdout(&["download", "add", &format!("magnet:?xt=urn:btih:{HASH}"), "movie", "tmdb:10"]);
    let failed = setup.stdout(&["import", "run"]);
    let listed = setup.stdout(&["import", "list"]);
    std::fs::remove_file(&blocker).unwrap();
    let import = listed.split_whitespace().next().unwrap();
    let retried = setup.stdout(&["import", "retry", import]);

    assert!(added.starts_with("Added Dune.2021.1080p; it is already complete\nImport of "), "{added}");
    assert!(added.trim_end().ends_with("already exists"), "{added}");
    assert_eq!(failed, "");
    assert!(listed.contains("failed") && listed.contains("already exists"), "{listed}");
    assert!(retried.starts_with("Imported "), "{retried}");
    assert_eq!(setup.stdout(&["import", "list"]), "No imports waiting.\n");
}
