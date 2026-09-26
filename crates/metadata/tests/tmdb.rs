use jiff::civil::date;
use rstest::rstest;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param, query_param_is_missing},
};
use yokoku_domain::{ExternalId, MediaKind, SourceStatus};
use yokoku_library::ports::{MetadataError, MetadataProvider};
use yokoku_metadata::TmdbClient;

const TOKEN: &str = "test-token";

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

async fn server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer, region: &str) -> TmdbClient {
    TmdbClient::new(TOKEN, "en-US", region).with_base_url(server.uri())
}

/// Serves `tv/{id}` for the season list and `tv/{id}?append_to_response=...` for the seasons.
async fn mount_series(server: &MockServer, id: u64, append: &str) {
    let endpoint = format!("/tv/{id}");
    Mock::given(method("GET"))
        .and(path(&endpoint))
        .and(query_param_is_missing("append_to_response"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture(&format!("tv_{id}.json"))))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(&endpoint))
        .and(query_param("append_to_response", append))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture(&format!("tv_{id}_seasons.json"))))
        .mount(server)
        .await;
}

#[tokio::test]
async fn search_returns_movies_and_series_but_not_people() {
    let server = server().await;
    Mock::given(method("GET"))
        .and(path("/search/multi"))
        .and(query_param("query", "dune"))
        .and(query_param("language", "en-US"))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("search_dune.json")))
        .mount(&server)
        .await;

    let results = client(&server, "US").search("dune").await.unwrap();

    let first: Vec<_> = results.iter().take(4).map(|r| (r.kind, r.source, r.title.as_str(), r.year)).collect();
    assert_eq!(
        first,
        [
            (MediaKind::Movie, ExternalId::Tmdb(438631), "Dune", Some(2021)),
            (MediaKind::Series, ExternalId::Tmdb(90228), "Dune: Prophecy", Some(2024)),
            (MediaKind::Movie, ExternalId::Tmdb(841), "Dune", Some(1984)),
            (MediaKind::Movie, ExternalId::Tmdb(693134), "Dune: Part Two", Some(2024)),
        ]
    );
    let expected = fixture("search_dune.json")["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["media_type"] != "person")
        .count();
    assert_eq!(results.len(), expected);
}

#[tokio::test]
async fn series_include_every_season_with_episodes() {
    let server = server().await;
    mount_series(&server, 209867, "season/0,season/1").await;

    let frieren = client(&server, "US").series(ExternalId::Tmdb(209867)).await.unwrap();

    assert_eq!(frieren.title, "Frieren: Beyond Journey's End");
    assert_eq!(frieren.original_title, "葬送のフリーレン");
    assert_eq!(frieren.year, Some(2023));
    assert_eq!(frieren.status, SourceStatus::Returning);
    assert_eq!(frieren.seasons.iter().map(|season| season.number).collect::<Vec<_>>(), [0, 1]);
    let first = &frieren.seasons[1].episodes[0];
    assert_eq!(
        (first.source_id, first.number, first.title.as_str(), first.air_date),
        (3946240, 1, "The Journey's End", Some(date(2023, 9, 29)))
    );
    let recorded = fixture("tv_209867_seasons.json");
    for season in &frieren.seasons {
        let episodes = recorded[format!("season/{}", season.number)]["episodes"].as_array().unwrap().len();
        assert_eq!(season.episodes.len(), episodes);
    }
}

#[tokio::test]
async fn ended_series_keep_their_specials() {
    let server = server().await;
    mount_series(&server, 19885, "season/0,season/1,season/2,season/3,season/4").await;

    let sherlock = client(&server, "US").series(ExternalId::Tmdb(19885)).await.unwrap();

    assert_eq!(sherlock.status, SourceStatus::Ended);
    assert_eq!(sherlock.seasons.len(), 5);
    assert_eq!(sherlock.seasons[0].number, 0);
    assert_eq!(sherlock.seasons[0].episodes.len(), 9);
}

#[tokio::test]
async fn long_series_load_seasons_twenty_at_a_time() {
    let server = server().await;
    let numbers: Vec<u16> = (0..=24).collect();
    let season = |number: u16| {
        json!({ "season_number": number, "episodes": [
            { "id": u64::from(number) * 100 + 1, "episode_number": 1, "name": format!("S{number}E1"), "air_date": "2001-01-01" }
        ]})
    };
    let details = json!({
        "name": "Long Show", "original_name": "Long Show", "first_air_date": "2001-01-01", "poster_path": null,
        "status": "Returning Series",
        "seasons": numbers.iter().map(|n| json!({ "season_number": n })).collect::<Vec<_>>(),
    });
    let with_seasons = |chunk: &[u16]| {
        let mut body = details.clone();
        for &number in chunk {
            body[format!("season/{number}")] = season(number);
        }
        body
    };
    let append = |chunk: &[u16]| chunk.iter().map(|n| format!("season/{n}")).collect::<Vec<_>>().join(",");
    Mock::given(path("/tv/1"))
        .and(query_param_is_missing("append_to_response"))
        .respond_with(ResponseTemplate::new(200).set_body_json(details.clone()))
        .mount(&server)
        .await;
    for chunk in numbers.chunks(20) {
        Mock::given(path("/tv/1"))
            .and(query_param("append_to_response", append(chunk)))
            .respond_with(ResponseTemplate::new(200).set_body_json(with_seasons(chunk)))
            .expect(1)
            .mount(&server)
            .await;
    }

    let series = client(&server, "US").series(ExternalId::Tmdb(1)).await.unwrap();

    assert_eq!(series.seasons.iter().map(|season| season.number).collect::<Vec<_>>(), numbers);
}

#[rstest]
#[case::region_dates("US", Some(date(2021, 10, 22)), Some(date(2021, 10, 21)), Some(date(2022, 1, 11)))]
#[case::primary_date_without_region("XX", Some(date(2021, 9, 15)), None, None)]
#[tokio::test]
async fn movies_take_release_dates_for_the_region(
    #[case] region: &str,
    #[case] cinema: Option<jiff::civil::Date>,
    #[case] digital: Option<jiff::civil::Date>,
    #[case] physical: Option<jiff::civil::Date>,
) {
    let server = server().await;
    Mock::given(path("/movie/438631"))
        .and(query_param("append_to_response", "release_dates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;

    let dune = client(&server, region).movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!((dune.title.as_str(), dune.year), ("Dune", Some(2021)));
    assert_eq!((dune.releases.cinema, dune.releases.digital, dune.releases.physical), (cinema, digital, physical));
}

#[tokio::test]
async fn missing_items_are_not_found() {
    let server = server().await;
    Mock::given(path("/tv/404")).respond_with(ResponseTemplate::new(404)).mount(&server).await;

    let error = client(&server, "US").series(ExternalId::Tmdb(404)).await.unwrap_err();

    assert!(matches!(error, MetadataError::NotFound(ExternalId::Tmdb(404))));
}

#[rstest]
#[case::bad_token(401)]
#[case::rate_limited(429)]
#[case::server_error(500)]
#[tokio::test]
async fn failing_requests_leave_the_source_unavailable(#[case] status: u16) {
    let server = server().await;
    Mock::given(path("/movie/1")).respond_with(ResponseTemplate::new(status)).mount(&server).await;

    let error = client(&server, "US").movie(ExternalId::Tmdb(1)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Unavailable(_)));
}

#[tokio::test]
async fn tvdb_ids_are_not_looked_up_on_tmdb() {
    let server = server().await;

    let error = client(&server, "US").series(ExternalId::Tvdb(81189)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Unavailable(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}
