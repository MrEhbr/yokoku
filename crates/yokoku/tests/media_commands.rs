use std::{fs, path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};
use predicates::prelude::*;
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_domain::{EpisodeMetadata, ExternalId, MonitorPreset, SeasonMetadata, Series, SeriesMetadata, SourceStatus};
use yokoku_library::ports::SeriesRepo;

/// A database with "Frieren" (tmdb:1), two episodes aired a week ago, and a series root `tv`.
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
            air_date: Some(week_ago),
        };
        let frieren = SeriesMetadata {
            source: ExternalId::Tmdb(1),
            title: "Frieren".into(),
            original_title: "Sousou no Frieren".into(),
            alternate_titles: Vec::new(),
            year: Some(2023),
            poster_path: None,
            status: SourceStatus::Returning,
            seasons: vec![SeasonMetadata { number: 1, episodes: vec![episode(1), episode(2)] }],
        };
        SeriesRepo::save(&db, &mut Series::add(frieren, MonitorPreset::All, today(), Timestamp::now()), &[])
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

    fn episode_line(&self, reference: &str) -> String {
        let stdout = self.stdout(&["show", "series", "tmdb:1"]);
        stdout.lines().find(|line| line.trim_start().starts_with(reference)).unwrap().to_owned()
    }
}

fn today() -> Date {
    Timestamp::now().to_zoned(TimeZone::UTC).date()
}

#[tokio::test]
async fn root_folders_are_added_as_absolute_paths_listed_and_removed() {
    let setup = Setup::new().await;
    let tv = setup.path("tv").canonicalize().unwrap();

    let listed = setup.stdout(&["root", "list"]);
    let removed = setup.stdout(&["root", "remove", "tv"]);

    assert_eq!(listed, format!("series  {}\n", tv.display()));
    assert_eq!(removed, format!("Removed root {}\n", tv.display()));
    assert_eq!(setup.stdout(&["root", "list"]), "No root folders; add one with `yokoku root add`.\n");
}

#[tokio::test]
async fn scanned_files_mark_their_episodes_downloaded() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");

    let stdout = setup.stdout(&["scan"]);

    assert_eq!(stdout, "Linked 1 new files\n");
    assert!(setup.episode_line("S01E01").contains("downloaded"));
    assert!(setup.episode_line("S01E02").contains("missing"));
}

#[tokio::test]
async fn unrecognised_files_are_matched_through_review() {
    let setup = Setup::new().await;
    setup.write("tv/Unsorted/clip.mkv");
    assert!(setup.stdout(&["scan"]).contains("1 folders need review"));
    let listed = setup.stdout(&["review", "list"]);
    let import = listed.split_whitespace().next().unwrap().to_owned();

    setup.stdout(&["review", "match", &import, "1", "series", "tmdb:1", "S01E01-E02"]);
    let shown = setup.stdout(&["review", "show", &import]);
    let approved = setup.stdout(&["review", "approve", &import]);

    assert!(listed.contains("1 files") && listed.contains("Unsorted"), "{listed}");
    assert!(shown.contains("clip.mkv") && shown.contains("Frieren (2023) S01E01-E02"), "{shown}");
    assert_eq!(approved, "Linked 1 files\n");
    assert!(setup.episode_line("S01E02").contains("downloaded"));
    assert_eq!(setup.stdout(&["review", "list"]), "Nothing to review.\n");
}

#[tokio::test]
async fn a_series_match_needs_episodes() {
    let setup = Setup::new().await;
    setup.write("tv/Unsorted/clip.mkv");
    setup.stdout(&["scan"]);
    let listed = setup.stdout(&["review", "list"]);
    let import = listed.split_whitespace().next().unwrap();

    setup
        .command()
        .args(["review", "match", import, "1", "series", "tmdb:1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("A series match needs episodes"));
}

#[tokio::test]
async fn rename_previews_then_applies() {
    let setup = Setup::new().await;
    setup.write("tv/frieren/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);

    let preview = setup.stdout(&["rename"]);
    let applied = setup.stdout(&["rename", "series", "tmdb:1", "--apply"]);

    assert_eq!(
        preview,
        "frieren/Frieren (2023) - S01E01.mkv\n  -> Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv\n\
         Run with --apply to rename 1 files.\n"
    );
    assert_eq!(applied, "Renamed 1 files\n");
    assert!(setup.path("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv").exists());
    assert_eq!(setup.stdout(&["rename"]), "Nothing to rename.\n");
    assert!(setup.episode_line("S01E01").contains("downloaded"));
}

#[tokio::test]
async fn history_lists_what_happened_newest_first() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);
    setup.stdout(&["rename", "--apply"]);

    let all = setup.stdout(&["history"]);
    let frieren = setup.stdout(&["history", "series", "tmdb:1", "--limit", "1"]);

    let lines: Vec<_> = all.lines().collect();
    assert!(lines[0].contains("  Renamed "), "{all}");
    assert!(lines[1].trim_start().starts_with("-> ") && lines[1].ends_with("S01E01 - Episode 1.mkv"), "{all}");
    assert!(lines[2].contains("  Found ") && lines[2].ends_with("Frieren (2023) - S01E01.mkv"), "{all}");
    assert_eq!(frieren.lines().count(), 2, "{frieren}");
    assert!(frieren.contains("Renamed "), "{frieren}");
}

#[tokio::test]
async fn deleting_an_episode_recycles_its_file_and_marks_it_missing() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);
    let recycle = setup.path("recycle");

    let output = setup
        .command()
        .args(["delete", "series", "tmdb:1", "S01E01"])
        .env("APP__RECYCLE__FOLDER", &recycle)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8(output.stdout).unwrap().starts_with("Recycled "));
    assert!(setup.episode_line("S01E01").contains("missing"));
    let day = std::fs::read_dir(&recycle).unwrap().next().unwrap().unwrap().path();
    assert!(day.join("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv").exists());
}

#[tokio::test]
async fn removing_a_series_with_its_files_deletes_them() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);

    setup.stdout(&["remove", "series", "tmdb:1", "--delete-files"]);

    assert!(!setup.path("tv/Frieren (2023)").exists());
    let history = setup.stdout(&["history", "-n", "2"]);
    assert!(history.contains("Deleted ") && history.contains("(its item was removed)"), "{history}");
}

#[tokio::test]
async fn changing_library_files_asks_jellyfin_to_rescan() {
    let setup = Setup::new().await;
    let jellyfin = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/Library/Refresh"))
        .and(wiremock::matchers::header("Authorization", "MediaBrowser Token=\"key\""))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .expect(1)
        .mount(&jellyfin)
        .await;
    wiremock::Mock::given(wiremock::matchers::path("/System/Info"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({ "Version": "10.10.7" })))
        .mount(&jellyfin)
        .await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);
    let with_jellyfin = |args: &[&str]| {
        let output = setup
            .command()
            .args(args)
            .env("APP__JELLYFIN__URL", jellyfin.uri())
            .env("APP__JELLYFIN__API_KEY", "key")
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };

    let tested = with_jellyfin(&["jellyfin", "test"]);
    with_jellyfin(&["delete", "series", "tmdb:1", "S01E01"]);
    with_jellyfin(&["list"]);

    assert_eq!(tested, "Connected to Jellyfin 10.10.7\n");
}

#[tokio::test]
async fn an_unreachable_jellyfin_does_not_fail_the_change() {
    let setup = Setup::new().await;
    setup.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);

    setup
        .command()
        .args(["delete", "series", "tmdb:1", "S01E01"])
        .env("APP__JELLYFIN__URL", "http://127.0.0.1:9")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Deleted "));
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
async fn renames_follow_the_configured_patterns() {
    let setup = Setup::new().await;
    setup.write("tv/frieren/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);

    let preview = setup
        .command()
        .arg("rename")
        .env("APP__NAMING__SEASON_FOLDER", "S{season}")
        .env("APP__NAMING__EPISODE_FILE", "{episodes} {episode_title}")
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8(preview.stdout).unwrap(),
        "frieren/Frieren (2023) - S01E01.mkv\n  -> Frieren (2023)/S01/S01E01 Episode 1.mkv\nRun with --apply to rename 1 files.\n"
    );
}

#[tokio::test]
async fn an_invalid_pattern_is_refused_with_its_reason() {
    let setup = Setup::new().await;

    setup.command().arg("rename").env("APP__NAMING__EPISODE_FILE", "{title}").assert().failure().stderr(
        predicate::str::contains("Invalid [naming] setting").and(predicate::str::contains("episode file pattern")),
    );
}

#[tokio::test]
async fn stored_settings_apply_to_every_command() {
    let setup = Setup::new().await;
    setup.write("tv/frieren/Frieren (2023) - S01E01.mkv");
    setup.stdout(&["scan"]);

    setup.stdout(&["settings", "set", "naming.season_folder", "S{season}"]);
    let preview = setup.stdout(&["rename"]);

    assert!(preview.contains("-> Frieren (2023)/S01/Frieren (2023) - S01E01 - Episode 1.mkv"), "{preview}");
}

#[tokio::test]
async fn a_stored_value_that_no_longer_loads_can_still_be_unset() {
    let setup = Setup::new().await;
    let db = Database::open(&setup.database).await.unwrap();
    db.set_setting("import.mode", &serde_json::json!("sideways")).await.unwrap();

    setup.command().arg("list").assert().failure().stderr(predicate::str::contains("see `yokoku settings list`"));
    let unset = setup.stdout(&["settings", "unset", "import.mode"]);

    assert_eq!(unset, "Unset import.mode\n");
    setup.command().arg("list").assert().success();
}
