use std::{fs, path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};
use predicates::prelude::*;
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, EpisodeRef, ExternalId, ItemFolder, MonitorPreset, Movie, MovieMetadata,
    Releases, SeasonMetadata, Series, SeriesMetadata, SettingsStore, SourceStatus,
};
use yokoku_events::EventLog;
use yokoku_library::ports::{MovieRepo, SeriesRepo};
use yokoku_test_support::{
    events::{accept_events, refuse_events},
    metadata::movie_metadata,
};

/// A database with "Frieren" (tmdb:1) in `tv/Frieren (2023)`, two episodes aired a week ago, and a series
/// root `tv`.
struct Setup {
    dir: TempDir,
    database: PathBuf,
}

impl Setup {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("yokoku.db");
        let db = Database::open(&database).await.unwrap();
        let week_ago = today() - 7.days();
        let episode = |number| EpisodeMetadata {
            source_id: number,
            number: number as u16,
            title: format!("Episode {number}"),
            overview: String::new(),
            air_date: Some(week_ago),
        };
        let tv = dir.path().canonicalize().unwrap().join("tv");
        let folder = ItemFolder::new(tv, "Frieren (2023)".into()).unwrap();
        let frieren = SeriesMetadata {
            source: ExternalId::Tmdb(1),
            title: "Frieren".into(),
            original_title: "Sousou no Frieren".into(),
            alternate_titles: Vec::new(),
            year: Some(2023),
            artwork: Artwork::default(),
            description: Description::default(),
            status: SourceStatus::Returning,
            seasons: vec![SeasonMetadata { number: 1, episodes: vec![episode(1), episode(2)] }],
        };
        SeriesRepo::save(&db, &mut Series::add(frieren, folder, MonitorPreset::All, today(), Timestamp::now()))
            .await
            .unwrap();

        let setup = Self { dir, database };
        fs::create_dir(setup.path("tv")).unwrap();
        setup.stdout(&["root", "add", "series", "tv"]);
        setup
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn write(&self, relative: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"video").unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command
            .current_dir(self.dir.path())
            .env("APP__DATABASE__PATH", &self.database)
            .env("APP__CLOCK__TIMEZONE", "UTC");
        command
    }

    fn stdout(&self, args: &[&str]) -> String {
        let output = self.command().args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }

    /// Whether "Frieren"'s `season`/`episode` has a linked file.
    async fn episode_downloaded(&self, season: u16, episode: u16) -> bool {
        let db = Database::open(&self.database).await.unwrap();
        let series = SeriesRepo::find_by_source(&db, ExternalId::Tmdb(1)).await.unwrap().unwrap();
        series.episode(EpisodeRef { season, episode }).unwrap().file.is_some()
    }
}

fn today() -> Date {
    Timestamp::now().to_zoned(TimeZone::UTC).date()
}

#[tokio::test]
async fn root_folders_are_added_as_absolute_paths_listed_and_removed() {
    let setup = Setup::new().await;
    let tv = setup.path("tv").canonicalize().unwrap();

    let films = tv.with_file_name("films");
    fs::create_dir(setup.path("films")).unwrap();
    let added = setup.stdout(&["root", "add", "movies", "films"]);
    let listed = setup.stdout(&["root", "list"]);
    let removed = setup.stdout(&["root", "remove", "films"]);

    assert_eq!(added, format!("Added movies root {}\n", films.display()));
    assert_eq!(listed, format!("movies  {}\nseries  {}\n", films.display(), tv.display()));
    assert_eq!(removed, format!("Removed root {}\n", films.display()));
    assert_eq!(setup.stdout(&["root", "list"]), format!("series  {}\n", tv.display()));
}

#[tokio::test]
async fn a_scan_records_its_files_and_leaves_their_handling_to_the_service() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");

    let stdout = setup.stdout(&["scan"]);

    assert_eq!(stdout, "Linked 1 new files\n");
    let log = EventLog::new(Database::open(&setup.database).await.unwrap().pool().clone());
    let events: Vec<&str> =
        log.read_after(None, 10).await.unwrap().iter().map(|recorded| recorded.event.name()).collect();
    assert_eq!(events, ["FilesFound"]);
    assert!(!setup.episode_downloaded(1, 1).await, "no handler runs outside the service");
}

#[tokio::test]
async fn a_command_whose_events_cannot_be_recorded_fails_after_saving_its_change() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    let db = Database::open(&setup.database).await.unwrap();
    refuse_events(&db).await;

    let scan = setup.command().arg("scan").assert().failure();
    accept_events(&db).await;

    scan.stderr(predicate::str::contains("its events could not be recorded"));
    assert_eq!(setup.stdout(&["scan"]), "Linked 0 new files\n");
}

#[tokio::test]
async fn the_jellyfin_api_key_can_be_read_from_a_file() {
    let setup = Setup::new().await;
    let jellyfin = wiremock::MockServer::start().await;
    yokoku_test_support::jellyfin::system_info("key", "10.10.7").expect(1).mount(&jellyfin).await;
    let key_file = setup.path("jellyfin.key");
    fs::write(&key_file, "key\n").unwrap();

    setup
        .command()
        .args(["jellyfin", "test"])
        .env("APP__JELLYFIN__URL", jellyfin.uri())
        .env("APP__JELLYFIN__API_KEY__FILE", &key_file)
        .assert()
        .success()
        .stdout("Connected to Jellyfin 10.10.7\n");
}

/// A stand-in for ffprobe that reports the recorded sample: 320x180 h264, English and Japanese
/// audio, and forced Russian subtitles.
fn stand_in_ffprobe(dir: &std::path::Path) -> PathBuf {
    let report = format!("{}/../system/tests/fixtures/ffprobe_sample.json", env!("CARGO_MANIFEST_DIR"));
    let program = dir.join("ffprobe");
    fs::write(&program, format!("#!/bin/sh\ncat '{report}'\n")).unwrap();
    fs::set_permissions(&program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    program
}

#[tokio::test]
async fn found_files_are_probed_and_shown_with_their_details() {
    let setup = Setup::new().await;
    let episode = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv";
    setup.write(episode);
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.en.sdh.srt");
    let ffprobe = stand_in_ffprobe(setup.dir.path());

    setup.command().arg("scan").env("APP__FILES__FFPROBE", &ffprobe).assert().success();
    setup.command().args(["files", "probe"]).env("APP__FILES__FFPROBE", &ffprobe).assert().success();
    let shown = setup.stdout(&["files", "show", "series", "tmdb:1"]);

    let path = setup.path("tv").canonicalize().unwrap().join("Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    assert_eq!(
        shown,
        format!(
            "{}\n  0.0 GB, 0h 00m, 320x180 h264\n  Audio      eng aac stereo, jpn aac mono\n  Subtitles  rus (forced) in the file; en (SDH) beside it\n",
            path.display()
        )
    );
}

#[tokio::test]
async fn files_are_probed_on_request_once_ffprobe_is_there() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    let missing = [("APP__FILES__FFPROBE", "/nonexistent/ffprobe")];
    setup.command().arg("scan").envs(missing).assert().success();

    let unprobed = setup.command().args(["files", "show", "series", "tmdb:1"]).envs(missing).output().unwrap();
    let failure = setup.command().args(["files", "probe"]).envs(missing).output().unwrap();
    let ffprobe = stand_in_ffprobe(setup.dir.path());
    let probed = setup.command().args(["files", "probe"]).env("APP__FILES__FFPROBE", &ffprobe).output().unwrap();

    assert!(String::from_utf8(unprobed.stdout).unwrap().contains("not probed yet; run `yokoku files probe`"));
    assert!(String::from_utf8(failure.stderr).unwrap().contains("/nonexistent/ffprobe is not installed"));
    assert_eq!(String::from_utf8(probed.stdout).unwrap(), "Probed 1 files\n");
}

#[tokio::test]
async fn a_stored_value_that_no_longer_loads_can_still_be_unset() {
    let setup = Setup::new().await;
    let db = Database::open(&setup.database).await.unwrap();
    db.set_setting("import.mode", &serde_json::json!("sideways")).await.unwrap();

    setup.command().arg("scan").assert().failure().stderr(predicate::str::contains("see `yokoku settings list`"));
    let unset = setup.stdout(&["settings", "unset", "import.mode"]);

    assert_eq!(unset, "Unset import.mode\n");
    setup.command().arg("scan").assert().success();
}

#[tokio::test]
async fn a_movie_file_is_shown_with_its_details() {
    let setup = Setup::new().await;
    fs::create_dir(setup.path("films")).unwrap();
    let films = setup.path("films").canonicalize().unwrap();
    let db = Database::open(&setup.database).await.unwrap();
    let dune = MovieMetadata { year: Some(2021), ..movie_metadata(2, "Dune", Releases::default()) };
    let folder = ItemFolder::new(films.clone(), "Dune (2021)".into()).unwrap();
    MovieRepo::save(&db, &mut Movie::add(dune, folder, true, Timestamp::now())).await.unwrap();
    setup.stdout(&["root", "add", "movies", "films"]);
    setup.write("films/Dune (2021)/Dune (2021).mkv");
    let ffprobe = stand_in_ffprobe(setup.dir.path());

    setup.command().arg("scan").env("APP__FILES__FFPROBE", &ffprobe).assert().success();
    setup.command().args(["files", "probe"]).env("APP__FILES__FFPROBE", &ffprobe).assert().success();
    let shown = setup.stdout(&["files", "show", "movie", "tmdb:2"]);

    let path = films.join("Dune (2021)/Dune (2021).mkv");
    assert!(shown.starts_with(&format!("{}\n  0.0 GB, 0h 00m, 320x180 h264\n", path.display())), "{shown}");
}
