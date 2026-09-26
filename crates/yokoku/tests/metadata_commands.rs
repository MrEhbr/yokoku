use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{path, query_param},
};

struct Tmdb {
    server: MockServer,
    dir: TempDir,
    database: PathBuf,
}

fn fixture(name: &str) -> Value {
    let file = format!("{}/../metadata/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

async fn respond(server: &MockServer, endpoint: &str, append: &str, body: &str) {
    let mock = Mock::given(path(endpoint)).and(query_param("append_to_response", append));
    mock.respond_with(ResponseTemplate::new(200).set_body_json(fixture(body))).mount(server).await;
}

/// Serves the recorded search for "dune", Frieren and Dune, with root folders `tv` and `movies`.
async fn tmdb() -> Tmdb {
    let server = MockServer::start().await;
    Mock::given(path("/search/multi"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("search_dune.json")))
        .mount(&server)
        .await;
    respond(&server, "/tv/209867", "alternative_titles", "tv_209867.json").await;
    respond(&server, "/tv/209867", "season/0,season/1", "tv_209867_seasons.json").await;
    respond(&server, "/movie/438631", "release_dates,alternative_titles", "movie_438631.json").await;

    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("yokoku.db");
    let tmdb = Tmdb { server, dir, database };
    for (kind, root) in [("series", "tv"), ("movies", "movies")] {
        std::fs::create_dir(tmdb.path(root)).unwrap();
        tmdb.command().args(["root", "add", kind]).arg(tmdb.path(root)).assert().success();
    }
    tmdb
}

impl Tmdb {
    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// `yokoku add` into the root folder of the item's kind.
    fn add(&self, args: &[&str]) -> Command {
        let root = if args[0] == "series" { "tv" } else { "movies" };
        let mut command = self.command();
        command.arg("add").args(args).arg("--root").arg(self.path(root));
        command
    }

    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command
            .env("APP__DATABASE__PATH", &self.database)
            .env("APP__CLOCK__TIMEZONE", "UTC")
            .env("APP__METADATA__TMDB_TOKEN", "test-token")
            .env("APP__METADATA__TMDB_URL", self.server.uri());
        command
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn search_marks_items_already_in_the_library() {
    let tmdb = tmdb().await;
    tmdb.add(&["movie", "tmdb:438631"]).assert().success();

    let output = tmdb.command().args(["search", "dune"]).output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert!(
        lines[0].starts_with("movie  Dune (2021)")
            && lines[0].contains("tmdb:438631")
            && lines[0].ends_with("in library")
    );
    assert!(lines[1].starts_with("series Dune: Prophecy (2024)") && !lines[1].ends_with("in library"));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_puts_series_and_movies_in_the_library() {
    let tmdb = tmdb().await;

    tmdb.add(&["series", "tmdb:209867", "--monitor", "none"]).assert().success().stdout(format!(
        "Added series Frieren: Beyond Journey's End (2023) tmdb:209867 in {}\n",
        tmdb.path("tv/Frieren - Beyond Journey's End (2023)").display()
    ));
    tmdb.add(&["movie", "tmdb:438631"])
        .assert()
        .success()
        .stdout(format!("Added movie Dune (2021) tmdb:438631 in {}\n", tmdb.path("movies/Dune (2021)").display()));

    tmdb.command()
        .args(["show", "series", "tmdb:209867"])
        .assert()
        .success()
        .stdout(predicate::str::contains("S01E01").and(predicate::str::contains("The Journey's End")));
    tmdb.command()
        .args(["show", "movie", "tmdb:438631"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cinema    2021-10-22"));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_uses_a_given_folder_and_links_the_files_already_there() {
    let tmdb = tmdb().await;
    let video = tmdb.path("movies/dune/dune.2021.mkv");
    std::fs::create_dir_all(video.parent().unwrap()).unwrap();
    std::fs::write(&video, b"video").unwrap();

    tmdb.add(&["movie", "tmdb:438631", "--folder", "dune"])
        .assert()
        .success()
        .stdout(format!("Added movie Dune (2021) tmdb:438631 in {}\n", tmdb.path("movies/dune").display()));

    tmdb.command()
        .args(["show", "movie", "tmdb:438631"])
        .assert()
        .success()
        .stdout(predicate::str::contains("downloaded"));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_needs_a_root_folder_of_the_item_kind() {
    let tmdb = tmdb().await;

    tmdb.command()
        .args(["add", "movie", "tmdb:438631", "--root"])
        .arg(tmdb.path("tv"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("is a series root folder"));
    tmdb.command()
        .args(["add", "movie", "tmdb:438631", "--root"])
        .arg(tmdb.path("elsewhere"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a root folder"));
    tmdb.command().args(["add", "movie", "tmdb:438631"]).assert().failure().stderr(predicate::str::contains("--root"));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_rejects_duplicates_and_series_presets_for_movies() {
    let tmdb = tmdb().await;
    tmdb.add(&["series", "tmdb:209867"]).assert().success();

    tmdb.add(&["series", "tmdb:209867"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("tmdb:209867 is already in the library"));
    tmdb.add(&["movie", "tmdb:438631", "--monitor", "future"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("movies can only be monitored with `all` or `none`"));
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_updates_one_item_or_the_whole_library() {
    let tmdb = tmdb().await;
    tmdb.add(&["series", "tmdb:209867"]).assert().success();
    tmdb.add(&["movie", "tmdb:438631"]).assert().success();

    tmdb.command().args(["refresh"]).assert().success().stdout("Refreshed 2 items\n");
    tmdb.command().args(["refresh", "movie", "tmdb:438631"]).assert().success().stdout("Refreshed tmdb:438631\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn metadata_commands_need_a_token() {
    let tmdb = tmdb().await;

    tmdb.command()
        .env_remove("APP__METADATA__TMDB_TOKEN")
        .args(["search", "dune"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No TMDB token configured; set APP__METADATA__TMDB_TOKEN"));
}
