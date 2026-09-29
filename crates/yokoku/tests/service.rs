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
    Artwork, CorrelationId, Description, DownloadId, EpisodeMetadata, ExternalId, FileTarget, ImportId, ItemFolder,
    ItemId, MediaFileId, MonitorPreset, Movie, MovieId, MovieMetadata, Releases, SeasonMetadata, Series, SeriesId,
    SeriesMetadata, SourceStatus,
};
use yokoku_events::{Correlated, EventLog, FileRenamed, ImportFailed, MovieRemoved, TorrentAdded};
use yokoku_library::ports::{MovieRepo, SeriesRepo};
use yokoku_media::{
    AudioStream, MediaFile, MediaInfo, VideoStream,
    ports::{Changes, MediaRepo},
};

/// A running service; killed on drop.
struct Service {
    child: Child,
    port: u16,
}

impl Service {
    /// Serves the database in `dir` with empty web assets in `dir/public`; its log goes to
    /// `dir/service.log`.
    fn start(dir: &Path) -> Self {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        std::fs::create_dir_all(dir.join("public")).unwrap();
        let log = dir.join("service.log");
        let child = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
            .env("APP__DATABASE__PATH", dir.join("yokoku.db"))
            .env("DIOXUS_PUBLIC_PATH", dir.join("public"))
            .env("APP__WEB__PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log).unwrap())
            .spawn()
            .unwrap();
        let mut service = Self { child, port };
        let deadline = Instant::now() + Duration::from_secs(30);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            if let Some(status) = service.child.try_wait().unwrap() {
                panic!("the service exited with {status}:\n{}", std::fs::read_to_string(&log).unwrap());
            }
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

/// "Frieren" (tmdb:1), a series without a poster whose one episode, S01E01 "Departure", aired on
/// 2023-09-29 without a file, and "Dune" (tmdb:10), a movie with the poster `/dune.jpg` and a probed
/// 1080p file. Both have a description.
async fn seed(path: &Path) -> (SeriesId, MovieId) {
    let db = Database::open(path).await.unwrap();
    let now = Timestamp::now();
    let frieren = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        artwork: Artwork::default(),
        description: Description {
            overview: "An elf mage outlives her party.".into(),
            genres: vec!["Fantasy".into()],
            runtime: Some(25),
        },
        status: SourceStatus::Returning,
        seasons: vec![SeasonMetadata {
            number: 1,
            episodes: vec![EpisodeMetadata {
                source_id: 11,
                number: 1,
                title: "Departure".into(),
                overview: "The party returns to the capital.".into(),
                air_date: Some(jiff::civil::date(2023, 9, 29)),
            }],
        }],
    };
    let dune = MovieMetadata {
        source: ExternalId::Tmdb(10),
        title: "Dune".into(),
        original_title: "Dune".into(),
        alternate_titles: Vec::new(),
        year: Some(2021),
        artwork: Artwork { poster: Some("/dune.jpg".into()), ..Artwork::default() },
        description: Description { runtime: Some(155), ..Description::default() },
        releases: Releases::default(),
    };
    let today = now.to_zoned(jiff::tz::TimeZone::UTC).date();
    let mut series = Series::add(frieren, ItemFolder::default(), MonitorPreset::All, today, now);
    SeriesRepo::save(&db, &mut series).await.unwrap();
    let mut movie = Movie::add(dune, ItemFolder::default(), true, now);
    MovieRepo::save(&db, &mut movie).await.unwrap();
    let file = MediaFile {
        id: MediaFileId::generate(),
        path: "/movies/Dune (2021)/Dune (2021).mkv".into(),
        size: 1_430_000_000,
        target: FileTarget::Movie(movie.id),
        added_at: now,
    };
    MediaRepo::save(&db, &Changes { added_files: vec![file.clone()], ..Changes::default() }).await.unwrap();
    let video = VideoStream { codec: "h264".into(), width: 1920, height: 800 };
    let audio = AudioStream { codec: "eac3".into(), language: Some("eng".into()), channels: 6 };
    let info = MediaInfo { video: Some(video), audio: vec![audio], ..MediaInfo::default() };
    MediaRepo::save_media_info(&db, file.id, &info).await.unwrap();
    movie.file = Some(file.id);
    MovieRepo::save(&db, &mut movie).await.unwrap();
    (series.id, movie.id)
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

#[tokio::test]
async fn detail_pages_show_descriptions_episodes_releases_and_files() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let series = service.get(&format!("/series/{frieren}"));
    let movie = service.get(&format!("/movies/{dune}"));

    assert!(series.starts_with("HTTP/1.1 200"), "{series}");
    assert!(series.contains("Frieren") && series.contains("S01E01") && series.contains("Departure"), "{series}");
    assert!(movie.starts_with("HTTP/1.1 200"), "{movie}");
    assert!(movie.contains("Dune") && movie.contains("Releases"), "{movie}");
    for text in ["An elf mage outlives her party.", "Fantasy · 25 min per episode", "The party returns to the capital."]
    {
        assert!(series.contains(text), "{text}: {series}");
    }
    for text in ["2h 35m", "/movies/Dune (2021)/Dune (2021).mkv", "1.4 GB", "1080p · 1920x800 h264", "eng eac3 5.1"] {
        assert!(movie.contains(text), "{text}: {movie}");
    }
}

#[tokio::test]
async fn detail_pages_of_unknown_items_say_so() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let series = service.get(&format!("/series/{dune}"));
    let movie = service.get(&format!("/movies/{frieren}"));

    assert!(series.contains("Series not found"), "{series}");
    assert!(movie.contains("Movie not found"), "{movie}");
}

#[tokio::test]
async fn the_calendar_api_lists_the_monitored_releases_of_a_period() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let september = service.get("/api/calendar?period=month&day=2023-09-10");
    let week = service.get("/api/calendar?period=week&day=2023-09-10");

    assert!(september.starts_with("HTTP/1.1 200"), "{september}");
    assert!(september.contains("\"from\":\"2023-09-01\"") && september.contains("Departure"), "{september}");
    assert!(week.contains("\"from\":\"2023-09-04\"") && !week.contains("Departure"), "{week}");
}

#[tokio::test]
async fn the_missing_page_lists_aired_episodes_without_a_file() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let page = service.get("/missing");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("Frieren") && page.contains("Departure"), "{page}");
}

#[tokio::test]
async fn a_cached_poster_is_served_at_the_url_the_library_lists() {
    let dir = tempfile::tempdir().unwrap();
    let (_, dune) = seed(&dir.path().join("yokoku.db")).await;
    let cached = dir.path().join(format!("artwork/movie/{dune}"));
    std::fs::create_dir_all(&cached).unwrap();
    std::fs::write(cached.join("poster-dune.jpg"), "dune poster").unwrap();
    let service = Service::start(dir.path());
    let url = format!("/artwork/movie/{dune}/poster/dune.jpg");

    let movies = service.get("/api/library?kind=movie");
    let poster = service.get(&url);

    assert!(movies.contains(&url), "{movies}");
    assert!(poster.starts_with("HTTP/1.1 200"), "{poster}");
    let headers = poster.to_ascii_lowercase();
    assert!(headers.contains("content-type: image/jpeg") && headers.contains("immutable"), "{poster}");
    assert!(poster.ends_with("dune poster"), "{poster}");
}

#[tokio::test]
async fn missing_artwork_unknown_items_and_unknown_kinds_are_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    for url in [
        format!("/artwork/series/{frieren}/poster/a.jpg"),
        format!("/artwork/movie/{dune}/logo/a.png"),
        format!("/artwork/movie/{frieren}/poster/a.jpg"),
        format!("/artwork/movie/{dune}/banner/a.jpg"),
        "/artwork/show/1/poster/a.jpg".into(),
    ] {
        let response = service.get(&url);
        assert!(response.starts_with("HTTP/1.1 404"), "{url}: {response}");
    }
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

/// Logs a torrent added for `dune`, an import of an unknown folder that failed, the rename of
/// `dune`'s file and the removal of "Arrival".
async fn seed_history(path: &Path, dune: MovieId) {
    let db = Database::open(path).await.unwrap();
    let correlation = CorrelationId::generate();
    let events = [
        TorrentAdded {
            download: DownloadId::generate(),
            name: "Dune.2021.1080p".into(),
            item: Some(ItemId::Movie(dune)),
        }
        .into(),
        ImportFailed {
            import: ImportId::generate(),
            source: "/downloads/Unknown".into(),
            reason: "nothing matched".into(),
        }
        .into(),
        FileRenamed {
            file: MediaFileId::generate(),
            from: "/movies/Dune/dune.mkv".into(),
            to: "/movies/Dune (2021)/Dune (2021).mkv".into(),
            target: Some(FileTarget::Movie(dune)),
        }
        .into(),
        MovieRemoved { movie: MovieId::generate(), title: "Arrival".into(), delete_files: false }.into(),
    ];
    let events = events.map(|event| Correlated { correlation, event });
    db.event_log().append(&events).await.unwrap();
}

#[tokio::test]
async fn the_history_page_lists_every_event_linked_to_its_item() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (_, dune) = seed(&path).await;
    seed_history(&path, dune).await;
    let service = Service::start(dir.path());

    let page = service.get("/history");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    for text in [
        "Added torrent Dune.2021.1080p",
        "An import failed: nothing matched",
        "/downloads/Unknown",
        "the file",
        "/movies/Dune/dune.mkv",
        "Removed movie ",
    ] {
        assert!(page.contains(text), "{text}: {page}");
    }
    let link = format!("href=\"/movies/{dune}\"");
    assert_eq!(page.matches(&link).count(), 2, "the torrent and the rename link to Dune: {page}");
    assert!(page.contains(">Arrival<") && !page.contains("Arrival</a>"), "{page}");
}

#[tokio::test]
async fn detail_pages_list_only_their_items_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    seed_history(&path, dune).await;
    let service = Service::start(dir.path());

    let movie = service.get(&format!("/movies/{dune}"));
    let series = service.get(&format!("/series/{frieren}"));

    assert!(movie.contains("Added torrent Dune.2021.1080p") && !movie.contains("/downloads/Unknown"), "{movie}");
    assert!(!movie.contains(&format!("href=\"/movies/{dune}\"")), "no link to the page itself: {movie}");
    let own = service.get(&format!("/api/movies/{dune}/history"));
    let all = service.get("/api/history");
    assert!(own.contains(r#"{"text":"Renamed "},{"text":"the file"}]"#), "the page's item goes unnamed: {own}");
    assert!(all.contains(r#"{"text":"the file"},{"text":" of "}"#), "{all}");
    assert!(series.contains("Nothing has happened to this series yet."), "{series}");
}

#[tokio::test]
async fn item_history_pages_load_older_entries_from_the_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (_, dune) = seed(&path).await;
    let db = Database::open(&path).await.unwrap();
    let correlation = CorrelationId::generate();
    let events: Vec<_> = (0..11)
        .map(|n| Correlated {
            correlation,
            event: TorrentAdded {
                download: DownloadId::generate(),
                name: format!("Dune {n}"),
                item: Some(ItemId::Movie(dune)),
            }
            .into(),
        })
        .collect();
    db.event_log().append(&events).await.unwrap();
    drop(db);
    let service = Service::start(dir.path());

    let newest = service.get(&format!("/api/movies/{dune}/history"));
    let older = newest.split("\"older\":").nth(1).unwrap().split(['}', ',']).next().unwrap();
    let oldest = service.get(&format!("/api/movies/{dune}/history?before={older}"));

    assert_eq!(newest.matches("Added torrent").count(), 10, "{newest}");
    assert!(newest.contains("Added torrent Dune 10") && !newest.contains("Added torrent Dune 0\""), "{newest}");
    assert_eq!(oldest.matches("Added torrent").count(), 1, "{oldest}");
    assert!(oldest.contains("Added torrent Dune 0\"") && oldest.contains("\"older\":null"), "{oldest}");
}
