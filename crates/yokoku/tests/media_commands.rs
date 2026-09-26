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
            year: Some(2023),
            poster_path: None,
            status: SourceStatus::Returning,
            seasons: vec![SeasonMetadata { number: 1, episodes: vec![episode(1), episode(2)] }],
        };
        SeriesRepo::save(&db, &Series::add(frieren, MonitorPreset::All, today(), Timestamp::now()), &[]).await.unwrap();

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
