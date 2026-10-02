use std::{
    io::{BufRead, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};

use assert_cmd::prelude::*;
use jiff::Timestamp;
use predicates::prelude::*;
use yokoku_core::{
    downloads::{Download, DownloadState, TorrentStatus, ports::DownloadRepo},
    events::{Correlated, EventLog},
    library::ports::{MovieRepo, SeriesRepo},
    media::{
        AudioStream, Import, ImportRow, ImportStatus, MediaFile, MediaInfo, Resolution, RootFolder, RootKind, RowMatch,
        VideoStream,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{
    Artwork, Confidence, CorrelationId, Description, DownloadId, EpisodeMetadata, ExternalId, FileTarget, ImportId,
    ItemFolder, ItemId, MediaFileId, MonitorPreset, Movie, MovieId, MovieMetadata, Releases, SeasonMetadata, Series,
    SeriesId, SeriesMetadata, SourceStatus,
    events::{FileRenamed, FilesFound, ImportFailed, MovieRemoved, TorrentAdded},
};
use yokoku_infra::db::Database;

/// A config file whose `[serve]` schedules do not fire while a test runs: midnight on January 1st.
const NEVER: &str = r#"[serve]
sync_downloads = "0 0 0 1 1 *"
sync_active_downloads = "0 0 0 1 1 *"
execute_imports = "0 0 0 1 1 *"
rescan_media_server = "0 0 0 1 1 *"
sync_watched = "0 0 0 1 1 *"
refresh_metadata = "0 0 0 1 1 *"
scan_library = "0 0 0 1 1 *"
"#;

/// A running service; killed on drop.
struct Service {
    child: Child,
    port: u16,
}

impl Service {
    /// Serves the database in `dir` with empty web assets in `dir/public` and no metadata source
    /// keys; its log goes to `dir/service.log`.
    fn start(dir: &Path) -> Self {
        Self::start_with(dir, &[])
    }

    /// Like `start`, with the environment variables `env` set. Scheduled jobs run only when `env`
    /// or a stored setting gives them a schedule.
    fn start_with(dir: &Path, env: &[(&str, &str)]) -> Self {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        std::fs::create_dir_all(dir.join("public")).unwrap();
        std::fs::write(dir.join("app.toml"), NEVER).unwrap();
        let log = dir.join("service.log");
        let child = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
            .arg("--config")
            .arg(dir.join("app.toml"))
            .env("APP__DATABASE__PATH", dir.join("yokoku.db"))
            .env("DIOXUS_PUBLIC_PATH", dir.join("public"))
            .env("APP__WEB__PORT", port.to_string())
            .env_remove("APP__METADATA__TMDB__TOKEN")
            .env_remove("APP__METADATA__TVDB__API_KEY")
            .envs(env.iter().copied())
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
        self.request(&format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"))
    }

    /// The response to an empty `POST path`.
    fn post(&self, path: &str) -> String {
        self.request(&format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ))
    }

    /// The response to `POST path` with the JSON `body`.
    fn post_json(&self, path: &str, body: &str) -> String {
        self.request(&format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    }

    fn request(&self, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
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
    let mut series = Series::new(frieren, ItemFolder::default(), MonitorPreset::All, today, now);
    SeriesRepo::save(&db, &mut series).await.unwrap();
    let mut movie = Movie::new(dune, ItemFolder::default(), true, now);
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
    for text in
        ["2h 35m", "/movies/Dune (2021)/Dune (2021).mkv", "1.4 GB on disk", "1080p · 1920x800 h264", "eng eac3 5.1"]
    {
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
async fn the_wanted_page_lists_aired_episodes_without_a_file() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let page = service.get("/wanted");
    let missing = service.get("/api/missing");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("Frieren") && page.contains("1 episode missing"), "{page}");
    assert!(missing.contains("Departure"), "{missing}");
}

#[tokio::test]
async fn an_unmonitored_episode_is_no_longer_missing() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, _) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());
    let episode = |season| {
        format!(r#"{{"target":{{"kind":"episode","id":"{frieren}","season":{season},"episode":1}},"monitored":false}}"#)
    };

    let off = service.post_json("/api/monitoring", &episode(1));
    let unknown = service.post_json("/api/monitoring", &episode(9));
    let missing = service.get("/wanted");

    assert!(off.starts_with("HTTP/1.1 200"), "{off}");
    assert!(!unknown.starts_with("HTTP/1.1 200") && unknown.contains("no longer lists it"), "{unknown}");
    assert!(missing.starts_with("HTTP/1.1 200") && !missing.contains("Departure"), "{missing}");
}

#[tokio::test]
async fn the_wanted_count_drops_when_an_episode_is_unmonitored() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, _) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());
    let count = || {
        let response = service.get("/api/missing/count");
        let body = response.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default();
        body.trim().parse::<usize>().unwrap_or_else(|_| panic!("{response}"))
    };
    let off = format!(r#"{{"target":{{"kind":"episode","id":"{frieren}","season":1,"episode":1}},"monitored":false}}"#);

    let before = count();
    service.post_json("/api/monitoring", &off);
    let after = count();

    assert!(before > 0);
    assert_eq!(after, before - 1);
}

#[tokio::test]
async fn a_series_can_switch_to_absolute_numbering() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, _) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let set = service.post_json(&format!("/api/series/{frieren}/numbering"), r#"{"numbering":"absolute"}"#);
    let series = service.get(&format!("/api/series/{frieren}"));

    assert!(set.starts_with("HTTP/1.1 200"), "{set}");
    assert!(series.contains(r#""numbering":"absolute""#), "{series}");
}

#[tokio::test]
async fn a_removed_item_leaves_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let (_, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());
    let body = format!(r#"{{"item":{{"Movie":"{dune}"}},"delete":[]}}"#);

    let removed = service.post_json("/api/items/remove", &body);
    let again = service.post_json("/api/items/remove", &body);
    let library = service.get("/api/library");

    assert!(removed.starts_with("HTTP/1.1 200"), "{removed}");
    assert!(again.contains("no longer in the library"), "{again}");
    assert!(library.contains("Frieren") && !library.contains("Dune"), "{library}");
}

#[tokio::test]
async fn deleting_a_file_that_is_gone_asks_for_a_reload() {
    let dir = tempfile::tempdir().unwrap();
    let (frieren, _) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let deleted = service.post_json(
        "/api/files/delete",
        &format!(r#"{{"target":{{"kind":"episode","id":"{frieren}","season":1,"episode":1}}}}"#),
    );

    assert!(!deleted.starts_with("HTTP/1.1 200") && deleted.contains("no file anymore"), "{deleted}");
}

#[tokio::test]
async fn a_rename_preview_says_which_files_stay() {
    let dir = tempfile::tempdir().unwrap();
    let (_, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());
    let item = format!(r#"{{"Movie":"{dune}"}}"#);

    let preview = service.post_json("/api/rename/preview", &format!(r#"{{"item":{item}}}"#));
    let renamed = service.post_json("/api/rename", &format!(r#"{{"item":{item},"files":[]}}"#));

    assert!(preview.starts_with("HTTP/1.1 200"), "{preview}");
    assert!(
        preview.contains(r#""renames":[]"#) && preview.contains("/movies/Dune (2021)/Dune (2021).mkv"),
        "{preview}"
    );
    assert!(preview.contains("not in a root folder"), "{preview}");
    assert!(renamed.contains(r#""renamed":0"#), "{renamed}");
}

#[tokio::test]
async fn settings_can_be_changed_and_reset() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::start(dir.path());

    let page = service.get("/settings");
    let saved = service.post_json("/api/settings", r#"{"key":"import.mode","value":"copy"}"#);
    let wrong = service.post_json("/api/settings", r#"{"key":"import.mode","value":"teleport"}"#);
    let hidden = service.post_json("/api/settings", r#"{"key":"web.port","value":9000}"#);
    let reset = service.post_json("/api/settings/reset", r#"{"key":"import.mode"}"#);

    assert!(
        page.starts_with("HTTP/1.1 200") && page.contains("Import mode") && page.contains("Listen address"),
        "{page}"
    );
    assert!(saved.contains(r#""value":"copy","stored":true"#), "{saved}");
    assert!(!wrong.starts_with("HTTP/1.1 200") && wrong.contains("teleport"), "{wrong}");
    assert!(hidden.contains("cannot be changed here"), "{hidden}");
    assert!(reset.contains(r#""value":"hardlink","stored":false"#), "{reset}");
}

#[tokio::test]
async fn root_folders_can_be_added_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let shows = dir.path().join("shows");
    std::fs::create_dir(&shows).unwrap();
    let service = Service::start(dir.path());
    let path = shows.display().to_string();

    let added = service.post_json("/api/roots", &format!(r#"{{"kind":"series","path":"{path}"}}"#));
    let listed = service.get("/api/roots");
    let removed = service.post_json("/api/roots/remove", &format!(r#"{{"path":"{path}"}}"#));
    let relative = service.post_json("/api/roots", r#"{"kind":"movie","path":"movies"}"#);

    assert!(added.starts_with("HTTP/1.1 200"), "{added}");
    assert!(listed.contains(&format!(r#"{{"kind":"series","path":"{path}","items":0}}"#)), "{listed}");
    assert!(removed.starts_with("HTTP/1.1 200"), "{removed}");
    assert!(relative.contains("is not an absolute path"), "{relative}");
}

#[tokio::test]
async fn the_library_can_be_scanned_on_demand() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::start(dir.path());

    let scanned = service.post("/api/library/scan");

    assert!(scanned.starts_with("HTTP/1.1 200"), "{scanned}");
    assert!(scanned.contains(r#"{"found":0,"vanished":0,"unrecognised":0}"#), "{scanned}");
}

/// A movie in a `movies` root folder of `dir`'s database, with a file no scan has found yet.
async fn seed_unscanned_movie(dir: &Path) {
    let movies = dir.join("movies");
    let folder = ItemFolder::new(movies.clone(), "Dune (2021)".into()).unwrap();
    std::fs::create_dir_all(folder.path()).unwrap();
    std::fs::write(folder.path().join("Dune (2021).mkv"), b"video").unwrap();
    let db = Database::open(&dir.join("yokoku.db")).await.unwrap();
    MediaRepo::add_root_folder(&db, &RootFolder { kind: RootKind::Movies, path: movies }).await.unwrap();
    let dune = MovieMetadata {
        source: ExternalId::Tmdb(438631),
        title: "Dune".into(),
        original_title: "Dune".into(),
        alternate_titles: Vec::new(),
        year: Some(2021),
        artwork: Artwork::default(),
        description: Description::default(),
        releases: Releases::default(),
    };
    MovieRepo::save(&db, &mut Movie::new(dune, folder, true, Timestamp::now())).await.unwrap();
}

/// Waits up to 15 s for a scan of `dir`'s library to find a file.
async fn scanned(dir: &Path) -> bool {
    let log = EventLog::new(Database::open(&dir.join("yokoku.db")).await.unwrap());
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let events = log.read_after(None, 100).await.unwrap();
        if events.iter().any(|recorded| recorded.event.get::<FilesFound>().is_some()) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

#[tokio::test(flavor = "multi_thread")]
async fn scheduled_jobs_run_on_their_cron_schedule_until_the_service_stops() {
    let dir = tempfile::tempdir().unwrap();
    seed_unscanned_movie(dir.path()).await;
    let _service = Service::start_with(dir.path(), &[("APP__SERVE__SCAN_LIBRARY", "* * * * * *")]);

    assert!(scanned(dir.path()).await, "the library was not scanned on schedule");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schedule_changed_from_the_command_line_applies_while_the_service_runs() {
    let dir = tempfile::tempdir().unwrap();
    seed_unscanned_movie(dir.path()).await;
    let _service = Service::start_with(dir.path(), &[("APP__EVENTS__POLL_INTERVAL_MS", "100")]);

    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .args(["settings", "set", "serve.scan_library", "* * * * * *"])
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .assert()
        .success();

    assert!(scanned(dir.path()).await, "the library was not scanned on the changed schedule");
}

#[tokio::test]
async fn refreshing_says_why_it_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let (_, dune) = seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start(dir.path());

    let untokened = service.post_json("/api/items/refresh", &format!(r#"{{"item":{{"Movie":"{dune}"}}}}"#));
    let removed =
        service.post_json("/api/items/refresh", &format!(r#"{{"item":{{"Movie":"{}"}}}}"#, MovieId::generate()));

    assert!(untokened.contains("check the token"), "{untokened}");
    assert!(removed.contains("no longer in the library"), "{removed}");
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
    EventLog::new(db.clone()).append(&events).await.unwrap();
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
    EventLog::new(db.clone()).append(&events).await.unwrap();
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

/// The imports `seed_downloads` stores.
struct SeededImports {
    review: ImportId,
    failed: ImportId,
}

/// Torrents: one for `dune` 45% downloaded, one for Frieren's S01E01 whose import waits for
/// review, one whose import failed, one imported and seeding, and one no longer in the client.
/// Also a scan of Frieren's folder that found 2 files it could not recognise.
async fn seed_downloads(path: &Path, frieren: SeriesId, dune: MovieId) -> SeededImports {
    let db = Database::open(path).await.unwrap();
    let now = Timestamp::now();
    let status = |state, done| TorrentStatus {
        state,
        size: 1_000_000_000,
        done,
        download_rate: 2_400_000,
        eta: Some(3_900),
        download_dir: "/downloads".into(),
        error: None,
    };
    let download = |name: &str, status: TorrentStatus, item| Download {
        id: DownloadId::generate(),
        hash: name.to_lowercase(),
        name: name.into(),
        item,
        season: None,
        completed_at: (status.done == status.size).then_some(now),
        imported_at: None,
        status,
        added_at: now,
        revision: 0,
    };
    let downloading =
        download("Dune.2021.1080p", status(DownloadState::Downloading, 450_000_000), Some(ItemId::Movie(dune)));
    let review =
        download("Frieren.S01E01.1080p", status(DownloadState::Seeding, 1_000_000_000), Some(ItemId::Series(frieren)));
    let failed = download("Broken.Release", status(DownloadState::Stopped, 1_000_000_000), None);
    let imported = Download {
        imported_at: Some(now),
        ..download("Frieren.S01E02.1080p", status(DownloadState::Seeding, 1_000_000_000), Some(ItemId::Series(frieren)))
    };
    let gone = download("Gone.Torrent", status(DownloadState::Removed, 0), None);
    for download in [&downloading, &review, &failed, &imported, &gone] {
        DownloadRepo::save(&db, &mut download.clone()).await.unwrap();
    }

    let import = |source: &str, download: Option<DownloadId>, status, error: Option<&str>, files: u16| Import {
        id: ImportId::generate(),
        source: source.into(),
        download,
        status,
        error: error.map(Into::into),
        rows: (1..=files)
            .map(|n| ImportRow {
                path: format!("{source}/video {n}.mkv").into(),
                size: 7,
                matched: RowMatch::None,
                confidence: Confidence::Unknown,
                skipped: false,
                resolution: Resolution::Unresolved,
            })
            .collect(),
        created_at: now,
    };
    let imports = vec![
        import("/downloads/Frieren.S01E01.1080p", Some(review.id), ImportStatus::NeedsReview, None, 1),
        import("/downloads/Broken.Release", Some(failed.id), ImportStatus::Failed, Some("the disk is full"), 1),
        import(&ItemFolder::default().path().display().to_string(), None, ImportStatus::NeedsReview, None, 2),
    ];
    let seeded = SeededImports { review: imports[0].id, failed: imports[1].id };
    MediaRepo::save(&db, &Changes { imports, ..Changes::default() }).await.unwrap();
    seeded
}

#[tokio::test]
async fn the_downloads_page_shows_each_torrent_with_its_import() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    seed_downloads(&path, frieren, dune).await;
    let service = Service::start(dir.path());

    let page = service.get("/queue");

    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    for text in [
        "45%",
        "2 MB/s",
        "1h 05m",
        "Needs review",
        ">Review<",
        "Import failed",
        "the disk is full",
        ">Retry<",
        "Imported · seeding",
    ] {
        assert!(page.contains(text), "{text}: {page}");
    }
    assert!(!page.contains("Gone.Torrent"), "torrents no longer in the client are left out: {page}");
    assert!(page.contains(&format!("href=\"/movies/{dune}\"")), "{page}");
}

#[tokio::test]
async fn detail_pages_tell_how_many_files_a_scan_did_not_recognise() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    seed_downloads(&path, frieren, dune).await;
    let service = Service::start(dir.path());

    let series = service.get(&format!("/series/{frieren}"));

    assert!(
        series.contains("2 files in the folder weren&#39;t recognised"),
        "the scan's 2 files count, not the 1 of the torrent import for Frieren: {series}"
    );
}

#[tokio::test]
async fn a_failed_import_can_be_retried_and_one_waiting_for_review_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    let imports = seed_downloads(&path, frieren, dune).await;
    let service = Service::start(dir.path());

    let failed = service.post(&format!("/api/imports/{}/retry", imports.failed));
    let review = service.post(&format!("/api/imports/{}/retry", imports.review));

    assert!(failed.starts_with("HTTP/1.1 200"), "{failed}");
    assert!(!review.starts_with("HTTP/1.1 200") && review.contains("no longer waiting for a retry"), "{review}");
}

/// A Transmission that knows the session handshake and adds and lists one torrent, `Dune.2021.1080p`.
async fn transmission() -> wiremock::MockServer {
    use serde_json::json;
    use yokoku_test_support::transmission::{answer, server, success};
    let server = server().await;
    let hash = "0638ffbb73b3f3ef1ba1fbbfa05a7e1db69610f6";
    let torrent = json!({
        "hashString": hash, "name": "Dune.2021.1080p", "status": 4, "sizeWhenDone": 4_000_000_000u64,
        "leftUntilDone": 1_000_000_000u64, "rateDownload": 5_000_000, "eta": 600, "downloadDir": "/downloads",
        "error": 0, "errorString": "", "metadataPercentComplete": 1.0, "isFinished": false, "labels": ["yokoku"],
    });
    for (rpc, arguments) in [
        ("torrent-add", json!({ "torrent-added": { "hashString": hash, "id": 1, "name": "Dune.2021.1080p" } })),
        ("torrent-get", json!({ "torrents": [torrent] })),
    ] {
        answer(&server, rpc, success(arguments)).await;
    }
    server
}

#[tokio::test(flavor = "multi_thread")]
async fn a_torrent_can_be_added_for_an_item() {
    let transmission = transmission().await;
    let dir = tempfile::tempdir().unwrap();
    let (_, dune) = seed(&dir.path().join("yokoku.db")).await;
    let url = format!("{}/transmission/rpc", transmission.uri());
    let service = Service::start_with(dir.path(), &[("APP__TRANSMISSION__URL", &url)]);
    let magnet = |link: &str| format!(r#"{{"torrent":{{"magnet":"{link}"}},"item":{{"Movie":"{dune}"}}}}"#);

    let not_a_magnet = service.post_json("/api/downloads", &magnet("https://example.com/dune"));
    let added =
        service.post_json("/api/downloads", &magnet("magnet:?xt=urn:btih:0638ffbb73b3f3ef1ba1fbbfa05a7e1db69610f6"));
    let again =
        service.post_json("/api/downloads", &magnet("magnet:?xt=urn:btih:0638ffbb73b3f3ef1ba1fbbfa05a7e1db69610f6"));
    let downloads = service.get("/api/downloads");

    assert!(not_a_magnet.contains("starts with magnet:"), "{not_a_magnet}");
    assert!(added.starts_with("HTTP/1.1 200"), "{added}");
    assert!(again.contains("was already added"), "{again}");
    assert!(downloads.contains("Dune.2021.1080p") && downloads.contains(&dune.to_string()), "{downloads}");
}

#[tokio::test]
async fn adding_a_torrent_says_when_transmission_is_unreachable() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir.path().join("yokoku.db")).await;
    let service = Service::start_with(dir.path(), &[("APP__TRANSMISSION__URL", "http://127.0.0.1:9/transmission/rpc")]);

    let added = service.post_json("/api/downloads", r#"{"torrent":{"file":[100,56]},"item":null}"#);

    assert!(added.contains("Transmission could not be reached"), "{added}");
}

#[tokio::test(flavor = "multi_thread")]
async fn transmission_can_be_tested_from_settings() {
    use serde_json::json;
    use yokoku_test_support::transmission::{answer, server, success};

    let transmission = server().await;
    answer(&transmission, "session-get", success(json!({ "version": "4.1.3 (0)" }))).await;
    let url = format!("{}/transmission/rpc", transmission.uri());
    let dir = tempfile::tempdir().unwrap();
    let service = Service::start_with(dir.path(), &[("APP__TRANSMISSION__URL", &url)]);
    let unreachable_dir = tempfile::tempdir().unwrap();
    let unreachable = Service::start_with(
        unreachable_dir.path(),
        &[("APP__TRANSMISSION__URL", "http://127.0.0.1:9/transmission/rpc")],
    );

    let unsaved_dir = tempfile::tempdir().unwrap();
    let unsaved = Service::start(unsaved_dir.path());

    let tested = service.post_json("/api/settings/test", r#"{"connection":"transmission","changes":[]}"#);
    let failed = unreachable.post_json("/api/settings/test", r#"{"connection":"transmission","changes":[]}"#);
    let typed = unsaved.post_json(
        "/api/settings/test",
        &format!(r#"{{"connection":"transmission","changes":[["transmission.url","{url}"]]}}"#),
    );
    let stored = unsaved.get("/api/settings");

    assert!(tested.starts_with("HTTP/1.1 200") && tested.contains("Transmission 4.1.3 (0)"), "{tested}");
    assert!(!failed.starts_with("HTTP/1.1 200") && failed.contains("download client unavailable"), "{failed}");
    assert!(typed.starts_with("HTTP/1.1 200") && typed.contains("Transmission 4.1.3 (0)"), "{typed}");
    assert!(!stored.contains(&url), "a tested value is not stored: {stored}");
}

#[tokio::test]
async fn a_reviewed_download_is_matched_then_imported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    let imports = seed_downloads(&path, frieren, dune).await;
    let service = Service::start(dir.path());
    let import = format!(r#""import":"{}""#, imports.review);

    let before = service.post_json("/api/review", &format!("{{{import}}}"));
    let early = service.post_json("/api/review/approve", &format!("{{{import}}}"));
    let matched = service.post_json(
        "/api/review/match",
        &format!(
            r#"{{{import},"row":1,"target":{{"kind":"episodes","series":"{frieren}","season":1,"first":1,"last":1}}}}"#
        ),
    );
    let after = service.post_json("/api/review", &format!("{{{import}}}"));
    let approved = service.post_json("/api/review/approve", &format!("{{{import}}}"));
    let done = service.post_json("/api/review", &format!("{{{import}}}"));

    assert!(before.contains(r#""path":"video 1.mkv""#) && before.contains(r#""matched":null"#), "{before}");
    assert!(before.contains(r#""problem":"Not matched""#) && before.contains(r#""name":null"#), "{before}");
    assert!(early.contains("Match or uncheck file 1 first"), "{early}");
    assert!(matched.starts_with("HTTP/1.1 200"), "{matched}");
    assert!(after.contains(r#""title":"Frieren""#) && after.contains(r#""season":1"#), "{after}");
    assert!(after.contains(r#""matched":{"kind":"episodes""#) && after.contains(r#""problem":null"#), "{after}");
    assert!(after.contains(r#""name":"Season 01/Frieren"#), "{after}");
    assert!(approved.contains("queued"), "{approved}");
    assert!(done.ends_with("null"), "{done}");
}

#[tokio::test]
async fn the_live_downloads_stream_sends_the_downloads_then_each_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let (frieren, dune) = seed(&path).await;
    let imports = seed_downloads(&path, frieren, dune).await;
    let service = Service::start(dir.path());
    let mut stream = TcpStream::connect(("127.0.0.1", service.port)).unwrap();
    write!(stream, "GET /api/downloads/live HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut events =
        std::io::BufReader::new(stream).lines().map(Result::unwrap).filter(|line| line.starts_with("data:"));

    let first = events.next().unwrap();
    service.post(&format!("/api/imports/{}/retry", imports.failed));
    let second = events.next().unwrap();

    assert!(first.contains("Dune.2021.1080p") && first.contains("the disk is full"), "{first}");
    assert!(!second.contains("the disk is full"), "the retried import is no longer failed for that reason: {second}");
}

/// A TMDB server answering a movie search for "dune" with the recorded movies, and Dune
/// (tmdb:438631).
async fn tmdb() -> wiremock::MockServer {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{path, query_param},
    };
    use yokoku_test_support::metadata::fixture;
    let server = MockServer::start().await;
    let mut movies = fixture("search_dune.json");
    movies["results"].as_array_mut().unwrap().retain(|item| item["media_type"] == "movie");
    Mock::given(path("/search/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_json(movies))
        .mount(&server)
        .await;
    Mock::given(path("/movie/438631"))
        .and(query_param("append_to_response", "release_dates,alternative_titles,images"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;
    server
}

/// The value after `"in_library":` in the search hit of `source`.
fn in_library<'a>(search: &'a str, source: &str) -> &'a str {
    let hit = &search[search.find(&format!("\"source\":\"{source}\"")).expect(search)..];
    let value = &hit[hit.find("\"in_library\":").unwrap() + "\"in_library\":".len()..];
    &value[..value.find(['}', ',']).unwrap()]
}

#[tokio::test(flavor = "multi_thread")]
async fn a_search_result_can_be_added_to_a_root_folder() {
    let tmdb = tmdb().await;
    let dir = tempfile::tempdir().unwrap();
    let movies = dir.path().join("movies");
    std::fs::create_dir_all(movies.join("Dune (2021)")).unwrap();
    let db = Database::open(&dir.path().join("yokoku.db")).await.unwrap();
    MediaRepo::add_root_folder(&db, &RootFolder { kind: RootKind::Movies, path: movies.clone() }).await.unwrap();
    let uri = tmdb.uri();
    let service = Service::start_with(
        dir.path(),
        &[("APP__METADATA__TMDB__TOKEN", "test-token"), ("APP__METADATA__TMDB__URL", &uri)],
    );
    let root = movies.display().to_string();
    let item = format!(
        r#"{{"item":{{"kind":"movie","source":"tmdb:438631","root":"{root}","monitor":"all","folder":"Dune (2021)"}}}}"#
    );

    let page = service.get("/add?query=dune&kind=movie");
    let before = service.get("/api/search?query=dune&kind=movie");
    let options = service.get("/api/add-options");
    let added = service.post_json("/api/items", &item);
    let after = service.get("/api/search?query=dune&kind=movie");
    let again = service.post_json("/api/items", &item);
    let id = added.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default();
    let refreshed = service.post_json("/api/items/refresh", &format!(r#"{{"item":{id}}}"#));

    assert!(
        page.starts_with("HTTP/1.1 200") && page.contains("Dune: Part Two") && page.contains("Paul Atreides"),
        "{page}"
    );
    assert!(before.starts_with("HTTP/1.1 200"), "{before}");
    assert!(before.contains("/artwork/preview/tmdb:438631/poster?path=/v1tRXZ4JtD2Iv6fjkPvT4GiwslV.jpg"), "{before}");
    assert_eq!(in_library(&before, "tmdb:438631"), "null");
    assert!(
        before.contains(r#""overview":"Paul Atreides"#) && before.contains(r#""folder":"Dune (2021)""#),
        "{before}"
    );
    let movie_roots =
        format!(r#""movie_roots":[{{"path":"{root}","folders":["Dune (2021)"],"taken":[]}}],"monitor":"all""#);
    assert!(options.contains(&movie_roots), "{options}");
    assert!(added.starts_with("HTTP/1.1 200"), "{added}");
    assert_ne!(in_library(&after, "tmdb:438631"), "null", "{after}");
    assert_eq!(in_library(&after, "tmdb:841"), "null");
    assert!(service.get("/api/library?kind=movie").contains("Dune"));
    assert!(!again.starts_with("HTTP/1.1 200") && again.contains("It is already in the library"), "{again}");
    assert!(refreshed.starts_with("HTTP/1.1 200"), "{id}: {refreshed}");
}

#[tokio::test]
async fn searching_asks_for_a_tmdb_token_until_one_is_set() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::start(dir.path());

    let search = service.get("/api/search?query=dune&kind=movie");

    assert!(!search.starts_with("HTTP/1.1 200") && search.contains("Set a TMDB token"), "{search}");
}
