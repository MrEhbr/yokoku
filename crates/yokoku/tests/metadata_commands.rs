use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use jiff::{Timestamp, tz::TimeZone};
use predicates::prelude::*;
use tempfile::TempDir;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{path, query_param},
};
use yokoku_core::library::ports::{MovieRepo, SeriesRepo};
use yokoku_domain::{
    Artwork, Description, ExternalId, ExternalIds, ItemFolder, MonitorPreset, Movie, MovieMetadata, Releases,
    SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_infra::db::Database;
use yokoku_test_support::metadata::fixture;

struct Tmdb {
    server: MockServer,
    _dir: TempDir,
    database: PathBuf,
}

async fn respond(server: &MockServer, endpoint: &str, append: &str, body: &str) {
    let mock = Mock::given(path(endpoint)).and(query_param("append_to_response", append));
    mock.respond_with(ResponseTemplate::new(200).set_body_json(fixture(body))).mount(server).await;
}

/// A database with "Frieren" (tmdb:209867) and "Dune" (tmdb:438631) already in the library, and a
/// TMDB server that serves their current metadata.
async fn tmdb() -> Tmdb {
    let server = MockServer::start().await;
    respond(&server, "/tv/209867", "alternative_titles,images,external_ids", "tv_209867.json").await;
    respond(&server, "/tv/209867", "season/0,season/1", "tv_209867_seasons.json").await;
    respond(&server, "/movie/438631", "release_dates,alternative_titles,images", "movie_438631.json").await;

    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("yokoku.db");
    let db = Database::open(&database).await.unwrap();
    let now = Timestamp::now();
    let today = now.to_zoned(TimeZone::UTC).date();
    let frieren = SeriesMetadata {
        source: ExternalId::Tmdb(209867),
        external_ids: ExternalIds::default(),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        artwork: Artwork::default(),
        description: Description::default(),
        status: SourceStatus::Returning,
        seasons: vec![SeasonMetadata { number: 1, episodes: Vec::new() }],
    };
    let dune = MovieMetadata {
        source: ExternalId::Tmdb(438631),
        external_ids: ExternalIds::default(),
        title: "Dune".into(),
        original_title: "Dune".into(),
        alternate_titles: Vec::new(),
        year: Some(2021),
        artwork: Artwork::default(),
        description: Description::default(),
        releases: Releases::default(),
    };
    SeriesRepo::save(&db, &mut Series::new(frieren, ItemFolder::default(), MonitorPreset::All, today, now))
        .await
        .unwrap();
    MovieRepo::save(&db, &mut Movie::new(dune, ItemFolder::default(), true, now)).await.unwrap();

    Tmdb { server, _dir: dir, database }
}

impl Tmdb {
    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command
            .env("YOKOKU__DATABASE__PATH", &self.database)
            .env("YOKOKU__CLOCK__TIMEZONE", "UTC")
            .env("YOKOKU__METADATA__TMDB__TOKEN", "test-token")
            .env("YOKOKU__METADATA__TMDB__URL", self.server.uri())
            .env_remove("YOKOKU__METADATA__TVDB__API_KEY");
        command
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_updates_one_item_or_the_whole_library() {
    let tmdb = tmdb().await;

    tmdb.command().args(["refresh"]).assert().success().stdout("Refreshed 2 items\n");
    tmdb.command().args(["refresh", "movie", "tmdb:438631"]).assert().success().stdout("Refreshed tmdb:438631\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn metadata_commands_need_a_token() {
    let tmdb = tmdb().await;

    tmdb.command()
        .env_remove("YOKOKU__METADATA__TMDB__TOKEN")
        .args(["refresh"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No TMDB token configured; set YOKOKU__METADATA__TMDB__TOKEN"));
}
