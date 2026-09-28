use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};

use assert_cmd::prelude::*;
use jiff::Timestamp;
use predicates::prelude::*;
use yokoku_db::Database;
use yokoku_domain::{
    ExternalId, ItemFolder, MonitorPreset, Movie, MovieMetadata, Releases, Series, SeriesMetadata, SourceStatus,
};
use yokoku_library::ports::{MovieRepo, SeriesRepo};

/// A running service; killed on drop.
struct Service {
    child: Child,
    port: u16,
}

impl Service {
    fn start(dir: &Path) -> Self {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let child = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
            .env("APP__DATABASE__PATH", dir.join("yokoku.db"))
            .env("DIOXUS_PUBLIC_PATH", dir)
            .env("APP__WEB__PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let service = Self { child, port };
        let deadline = Instant::now() + Duration::from_secs(30);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(Instant::now() < deadline, "the service did not start listening");
            sleep(Duration::from_millis(50));
        }
        service
    }

    /// The response to `GET path`, status line and headers included.
    fn get(&self, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        _ = self.child.kill();
        _ = self.child.wait();
    }
}

/// "Frieren" (tmdb:1), a series, and "Dune" (tmdb:10), a movie.
async fn seed(path: &Path) {
    let db = Database::open(path).await.unwrap();
    let now = Timestamp::now();
    let frieren = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: Vec::new(),
    };
    let dune = MovieMetadata {
        source: ExternalId::Tmdb(10),
        title: "Dune".into(),
        original_title: "Dune".into(),
        alternate_titles: Vec::new(),
        year: Some(2021),
        poster_path: None,
        releases: Releases::default(),
    };
    let today = now.to_zoned(jiff::tz::TimeZone::UTC).date();
    let mut series = Series::add(frieren, ItemFolder::default(), MonitorPreset::All, today, now);
    SeriesRepo::save(&db, &mut series).await.unwrap();
    MovieRepo::save(&db, &mut Movie::add(dune, ItemFolder::default(), true, now)).await.unwrap();
}

#[tokio::test]
async fn the_library_page_lists_the_library() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let page = service.get("/");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("Frieren") && page.contains("Dune"), "{page}");
}

#[tokio::test]
async fn the_library_api_filters_by_type() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let movies = service.get("/api/library?kind=movie");

    assert!(movies.starts_with("HTTP/1.1 200"), "{movies}");
    assert!(movies.contains("Dune") && !movies.contains("Frieren"), "{movies}");
}

#[test]
fn the_service_stops_at_startup_without_web_assets() {
    let dir = tempfile::tempdir().unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .env("DIOXUS_PUBLIC_PATH", dir.path().join("public"))
        .env("APP__WEB__PORT", "0")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to start the web server")
                .and(predicate::str::contains("web assets not found")),
        );
}

#[test]
fn the_service_stops_at_startup_when_the_web_port_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .env("DIOXUS_PUBLIC_PATH", dir.path())
        .env("APP__WEB__PORT", taken.local_addr().unwrap().port().to_string())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to start the web server"));
}
