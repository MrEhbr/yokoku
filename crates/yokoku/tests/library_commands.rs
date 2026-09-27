use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};
use predicates::prelude::*;
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_domain::{
    EpisodeMetadata, ExternalId, ItemFolder, MonitorPreset, Movie, MovieMetadata, Releases, SeasonMetadata, Series,
    SeriesMetadata, SourceStatus,
};
use yokoku_events::{Event, EventLog, SeriesRemoved};
use yokoku_library::ports::{MovieRepo, SeriesRepo};

struct Library {
    _dir: TempDir,
    path: PathBuf,
}

impl Library {
    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command.env("APP__DATABASE__PATH", &self.path).env("APP__CLOCK__TIMEZONE", "UTC");
        command
    }

    fn stdout(&self, args: &[&str]) -> String {
        let output = self.command().args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }

    async fn events(&self) -> Vec<Event> {
        let db = Database::open(&self.path).await.unwrap();
        db.event_log().read_after(None, 100).await.unwrap().into_iter().map(|recorded| recorded.event).collect()
    }
}

fn today() -> Date {
    Timestamp::now().to_zoned(TimeZone::UTC).date()
}

async fn empty_library() -> Library {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    Database::open(&path).await.unwrap();
    Library { _dir: dir, path }
}

/// "Frieren" (tmdb:1) with an episode a week ago and one in a week; "Dune" (tmdb:10), released.
async fn seeded_library() -> Library {
    let library = empty_library().await;
    let db = Database::open(&library.path).await.unwrap();
    let episode = |source_id, number, air_date| EpisodeMetadata {
        source_id,
        number,
        title: format!("Episode {number}"),
        air_date: Some(air_date),
    };
    let frieren = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: vec![SeasonMetadata {
            number: 1,
            episodes: vec![episode(11, 1, today() - 7.days()), episode(12, 2, today() + 7.days())],
        }],
    };
    let dune = MovieMetadata {
        source: ExternalId::Tmdb(10),
        title: "Dune".into(),
        original_title: "Dune".into(),
        alternate_titles: Vec::new(),
        year: Some(2021),
        poster_path: None,
        releases: Releases { digital: Some(today() - 30.days()), ..Releases::default() },
    };

    SeriesRepo::save(
        &db,
        &mut Series::add(frieren, ItemFolder::default(), MonitorPreset::All, today(), Timestamp::now()),
    )
    .await
    .unwrap();
    MovieRepo::save(&db, &mut Movie::add(dune, ItemFolder::default(), true, Timestamp::now())).await.unwrap();
    library
}

#[tokio::test]
async fn list_shows_items_sorted_by_title() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["list"]);

    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].starts_with("Dune (2021)") && lines[0].contains("released") && lines[0].ends_with("tmdb:10"));
    assert!(lines[1].starts_with("Frieren (2023)") && lines[1].contains("continuing"));
    assert!(lines[1].contains(&(today() + 7.days()).to_string()));
}

#[tokio::test]
async fn list_filters_by_type() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["list", "--kind", "movie"]);

    assert!(stdout.contains("Dune"));
    assert!(!stdout.contains("Frieren"));
}

#[tokio::test]
async fn list_reports_an_empty_library() {
    let library = empty_library().await;

    library.command().arg("list").assert().success().stdout("No items.\n");
}

#[tokio::test]
async fn show_prints_episodes_with_their_status() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["show", "series", "tmdb:1"]);

    assert!(stdout.starts_with("Frieren (2023)  tmdb:1  continuing  monitored  numbering: standard"));
    assert!(stdout.contains("Season 1  monitored"));
    assert!(stdout.lines().any(|line| line.contains("S01E01") && line.contains("missing")));
    assert!(stdout.lines().any(|line| line.contains("S01E02") && line.contains("upcoming")));
}

#[tokio::test]
async fn show_rejects_items_not_in_the_library() {
    let library = seeded_library().await;

    library
        .command()
        .args(["show", "series", "tmdb:99"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("series tmdb:99 is not in the library"));
}

#[tokio::test]
async fn rejects_malformed_source_ids() {
    let library = seeded_library().await;

    library
        .command()
        .args(["show", "series", "frieren"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a source id like tmdb:1396"));
}

#[tokio::test]
async fn monitor_changes_the_chosen_level() {
    let library = seeded_library().await;

    library.command().args(["monitor", "series", "tmdb:1", "--season", "1", "--off"]).assert().success();
    library.command().args(["monitor", "series", "tmdb:1", "--season", "1", "--episode", "2"]).assert().success();

    let stdout = library.stdout(&["show", "series", "tmdb:1"]);
    assert!(stdout.contains("Season 1  unmonitored"));
    assert!(stdout.lines().any(|line| line.contains("S01E02") && line.contains(" monitored")));
}

#[tokio::test]
async fn monitor_rejects_seasons_for_movies() {
    let library = seeded_library().await;

    library
        .command()
        .args(["monitor", "movie", "tmdb:10", "--season", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("apply to series only"));
}

#[tokio::test]
async fn numbering_switches_to_absolute() {
    let library = seeded_library().await;

    library.command().args(["numbering", "tmdb:1", "absolute"]).assert().success();

    assert!(library.stdout(&["show", "series", "tmdb:1"]).contains("numbering: absolute"));
}

#[tokio::test]
async fn remove_deletes_the_item_and_records_it() {
    let library = seeded_library().await;

    library
        .command()
        .args(["remove", "series", "tmdb:1", "--delete-files"])
        .assert()
        .success()
        .stdout("Removed series tmdb:1\n");

    assert!(!library.stdout(&["list"]).contains("Frieren"));
    let events = library.events().await;
    assert!(matches!(
        events.as_slice(),
        [event] if event.get::<SeriesRemoved>().is_some_and(|removed| removed.title == "Frieren" && removed.delete_files)
    ));
}

#[tokio::test]
async fn show_prints_next_and_last_aired_episodes() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["show", "series", "tmdb:1"]);

    assert!(stdout.contains(&format!("Next      S01E02  {}  Episode 2", today() + 7.days())));
    assert!(stdout.contains(&format!("Last      S01E01  {}  missing", today() - 7.days())));
}

#[tokio::test]
async fn upcoming_lists_releases_within_the_window() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["upcoming"]);

    assert!(stdout.contains(&(today() + 7.days()).to_string()));
    assert!(
        stdout.lines().any(|line| line.contains("Frieren") && line.contains("S01E02") && line.contains("upcoming"))
    );
    library.command().args(["upcoming", "--days", "3"]).assert().success().stdout("Nothing scheduled.\n");
}

#[tokio::test]
async fn calendar_shows_the_week_or_month_of_a_date() {
    let library = seeded_library().await;
    let aired = today() - 7.days();

    let week = library.stdout(&["calendar", "--date", &aired.to_string()]);
    let month = library.stdout(&["calendar", "--month", "--date", &(today() - 30.days()).to_string()]);

    assert!(week.lines().any(|line| line.contains("S01E01") && line.contains("missing")));
    assert!(!week.contains("S01E02"));
    assert!(month.lines().any(|line| line.contains("Dune") && line.contains("digital release")));
}

#[tokio::test]
async fn missing_lists_aired_episodes_and_released_movies_without_files() {
    let library = seeded_library().await;

    let stdout = library.stdout(&["missing"]);

    assert_eq!(
        stdout,
        format!(
            "Frieren (2023)  tmdb:1\n  S01E01  {}  Episode 1\nMovies\n  Dune (2021)  tmdb:10\n",
            today() - 7.days()
        )
    );
}

#[tokio::test]
async fn missing_reports_when_nothing_is_missing() {
    let library = empty_library().await;

    library.command().arg("missing").assert().success().stdout("Nothing missing.\n");
}
