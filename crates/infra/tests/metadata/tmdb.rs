use jiff::civil::date;
use rstest::rstest;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};
use yokoku_core::library::ports::{MetadataError, MetadataProvider};
use yokoku_domain::{ExternalId, Live, MediaKind, Secret, SourceStatus};
use yokoku_infra::metadata::{MetadataSettings, TmdbClient, TmdbSettings};
use yokoku_test_support::metadata::fixture;

const TOKEN: &str = "test-token";

async fn server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer, region: &str) -> TmdbClient {
    let tmdb = TmdbSettings { token: Some(Secret::new(TOKEN)), url: server.uri() };
    TmdbClient::new(Live::fixed(MetadataSettings { region: region.into(), tmdb, ..MetadataSettings::default() }))
}

/// Serves `tv/{id}` with alternative titles for the season list, and `tv/{id}?append_to_response=...` for the seasons.
async fn mount_series(server: &MockServer, id: u64, append: &str) {
    let endpoint = format!("/tv/{id}");
    Mock::given(method("GET"))
        .and(path(&endpoint))
        .and(query_param("append_to_response", "alternative_titles,images,external_ids"))
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

    let results = client(&server, "US").search("dune", None).await.unwrap();

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

#[rstest]
#[case::series(MediaKind::Series, "/search/tv", json!({ "results": [
    {
        "id": 90228, "name": "Dune: Prophecy", "original_name": "Dune: Prophecy", "first_air_date": "2024-11-17",
        "overview": "The sisterhood.",
    },
]}))]
#[case::movies(MediaKind::Movie, "/search/movie", json!({ "results": [
    {
        "id": 438631, "title": "Dune", "original_title": "Dune", "release_date": "2021-09-15",
        "overview": "The sisterhood.",
    },
]}))]
#[tokio::test]
async fn a_search_for_one_kind_asks_for_that_kind_only(
    #[case] kind: MediaKind,
    #[case] endpoint: &str,
    #[case] page: Value,
) {
    let server = server().await;
    Mock::given(path(endpoint))
        .and(query_param("query", "dune"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page))
        .mount(&server)
        .await;

    let results = client(&server, "US").search("dune", Some(kind)).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!((results[0].kind, results[0].overview.as_str()), (kind, "The sisterhood."));
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
async fn series_take_their_description_and_episode_overviews() {
    let server = server().await;
    mount_series(&server, 209867, "season/0,season/1").await;

    let frieren = client(&server, "US").series(ExternalId::Tmdb(209867)).await.unwrap();

    assert!(frieren.description.overview.starts_with("After the party of heroes defeated the Demon King"));
    assert_eq!(frieren.description.genres, ["Animation", "Action & Adventure", "Drama", "Sci-Fi & Fantasy"]);
    assert_eq!(frieren.description.runtime, Some(25));
    assert!(frieren.seasons[1].episodes[0].overview.starts_with("The world celebrates the Demon King's"));
}

#[tokio::test]
async fn series_without_a_usual_runtime_take_their_most_common_episode_length() {
    let server = server().await;
    let mut details = fixture("tv_209867.json");
    details["episode_run_time"] = json!([]);
    let mut seasons = fixture("tv_209867_seasons.json");
    for season in ["season/0", "season/1"] {
        for (index, episode) in seasons[season]["episodes"].as_array_mut().unwrap().iter_mut().enumerate() {
            episode["runtime"] = json!(if index < 2 { 30 } else { 24 });
        }
    }
    Mock::given(path("/tv/209867"))
        .and(query_param("append_to_response", "alternative_titles,images,external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_json(details))
        .mount(&server)
        .await;
    Mock::given(path("/tv/209867"))
        .and(query_param("append_to_response", "season/0,season/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(seasons))
        .mount(&server)
        .await;

    let frieren = client(&server, "US").series(ExternalId::Tmdb(209867)).await.unwrap();

    assert_eq!(frieren.description.runtime, Some(24));
}

#[tokio::test]
async fn movies_take_their_description() {
    let server = server().await;
    Mock::given(path("/movie/438631"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert!(dune.description.overview.starts_with("Paul Atreides, a brilliant and gifted young man"));
    assert_eq!(dune.description.genres, ["Science Fiction", "Adventure"]);
    assert_eq!(dune.description.runtime, Some(155));
}

#[rstest]
#[case::known(json!("tt1160419"), Some("tt1160419"))]
#[case::empty(json!(""), None)]
#[case::missing(Value::Null, None)]
#[tokio::test]
async fn movies_take_their_imdb_id(#[case] imdb_id: Value, #[case] expected: Option<&str>) {
    let server = server().await;
    let mut movie = fixture("movie_438631.json");
    movie["imdb_id"] = imdb_id;
    Mock::given(path("/movie/438631"))
        .respond_with(ResponseTemplate::new(200).set_body_json(movie))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!(dune.external_ids.imdb, expected.map(|id| id.parse().unwrap()));
}

#[tokio::test]
async fn series_take_their_imdb_id_from_their_external_ids() {
    let server = server().await;
    let mut details = fixture("tv_209867.json");
    details["external_ids"] = json!({ "imdb_id": "tt22248376", "tvdb_id": 424536 });
    Mock::given(path("/tv/209867"))
        .and(query_param("append_to_response", "alternative_titles,images,external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_json(details))
        .mount(&server)
        .await;
    Mock::given(path("/tv/209867"))
        .and(query_param("append_to_response", "season/0,season/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("tv_209867_seasons.json")))
        .mount(&server)
        .await;

    let frieren = client(&server, "US").series(ExternalId::Tmdb(209867)).await.unwrap();

    assert_eq!(frieren.external_ids.imdb, Some("tt22248376".parse().unwrap()));
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
        .and(query_param("append_to_response", "alternative_titles,images,external_ids"))
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
        .and(query_param("append_to_response", "release_dates,alternative_titles,images"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;

    let dune = client(&server, region).movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!((dune.title.as_str(), dune.year), ("Dune", Some(2021)));
    assert_eq!((dune.releases.cinema, dune.releases.digital, dune.releases.physical), (cinema, digital, physical));
}

#[tokio::test]
async fn movies_take_the_backdrop_and_the_best_voted_logo_in_the_language() {
    let server = server().await;
    let mut movie = fixture("movie_438631.json");
    let logo = |path: &str, language: Option<&str>, votes: f64| json!({ "file_path": path, "iso_639_1": language, "vote_average": votes });
    movie["images"] = json!({ "logos": [
        logo("/de.png", Some("de"), 9.0),
        logo("/en-worse.png", Some("en"), 5.0),
        logo("/en.png", Some("en"), 7.5),
        logo("/textless.png", None, 8.0),
    ]});
    Mock::given(path("/movie/438631"))
        .and(query_param("include_image_language", "en,null"))
        .respond_with(ResponseTemplate::new(200).set_body_json(movie))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!(dune.artwork.poster.as_deref(), Some("/v1tRXZ4JtD2Iv6fjkPvT4GiwslV.jpg"));
    assert_eq!(dune.artwork.backdrop.as_deref(), Some("/zRKQW58MBEY078AxkHxEJzUskCl.jpg"));
    assert_eq!(dune.artwork.logo.as_deref(), Some("/en.png"));
}

#[tokio::test]
async fn a_logo_without_text_stands_in_for_one_in_the_language() {
    let server = server().await;
    let mut movie = fixture("movie_438631.json");
    movie["images"] = json!({ "logos": [
        { "file_path": "/de.png", "iso_639_1": "de", "vote_average": 9.0 },
        { "file_path": "/textless.png", "iso_639_1": null, "vote_average": 1.0 },
    ]});
    Mock::given(path("/movie/438631"))
        .respond_with(ResponseTemplate::new(200).set_body_json(movie))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!(dune.artwork.logo.as_deref(), Some("/textless.png"));
}

#[tokio::test]
async fn missing_items_are_not_found() {
    let server = server().await;
    Mock::given(path("/tv/404")).respond_with(ResponseTemplate::new(404)).mount(&server).await;

    let error = client(&server, "US").series(ExternalId::Tmdb(404)).await.unwrap_err();

    assert!(matches!(error, MetadataError::NotFound(ExternalId::Tmdb(404))), "{error:?}");
}

fn failing(status: u16) -> ResponseTemplate {
    ResponseTemplate::new(status).insert_header("retry-after", "0")
}

#[tokio::test]
async fn a_refused_token_is_not_retried_and_says_why() {
    let server = server().await;
    let body = json!({ "status_code": 7, "status_message": "Invalid API key: You must be granted a valid key." });
    Mock::given(path("/movie/1"))
        .respond_with(ResponseTemplate::new(401).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server, "US").movie(ExternalId::Tmdb(1)).await.unwrap_err();

    assert!(matches!(&error, MetadataError::Refused(reason) if reason.contains("Invalid API key")), "{error:?}");
}

#[rstest]
#[case::rate_limited(429)]
#[case::bad_gateway(502)]
#[case::overloaded(503)]
#[case::gateway_timeout(504)]
#[tokio::test]
async fn temporary_failures_are_retried(#[case] status: u16) {
    let server = server().await;
    Mock::given(path("/movie/438631")).respond_with(failing(status)).up_to_n_times(2).mount(&server).await;
    Mock::given(path("/movie/438631"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!(dune.title, "Dune");
}

#[rstest]
#[case::still_rate_limited(429, 3)]
#[case::server_error(500, 1)]
#[tokio::test]
async fn failing_requests_leave_the_source_unavailable(#[case] status: u16, #[case] attempts: u64) {
    let server = server().await;
    Mock::given(path("/movie/1")).respond_with(failing(status)).expect(attempts).mount(&server).await;

    let error = client(&server, "US").movie(ExternalId::Tmdb(1)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Unavailable(_)), "{error:?}");
}

#[tokio::test]
async fn answers_of_another_shape_are_invalid() {
    let server = server().await;
    Mock::given(path("/movie/1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>"))
        .mount(&server)
        .await;

    let error = client(&server, "US").movie(ExternalId::Tmdb(1)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Invalid(_)), "{error:?}");
}

#[tokio::test]
async fn tvdb_ids_are_not_looked_up_on_tmdb() {
    let server = server().await;

    let error = client(&server, "US").series(ExternalId::Tvdb(81189)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Unavailable(_)), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn series_keep_other_titles_once() {
    let server = server().await;
    mount_series(&server, 209867, "season/0,season/1").await;

    let frieren = client(&server, "US").series(ExternalId::Tmdb(209867)).await.unwrap();

    assert!(frieren.alternate_titles.contains(&"Sousou no Frieren".to_owned()));
    assert!(frieren.alternate_titles.contains(&"Провожающая в последний путь Фрирен".to_owned()));
    assert!(!frieren.alternate_titles.contains(&frieren.title));
    assert_eq!(frieren.alternate_titles.iter().filter(|title| *title == "葬送的芙莉莲").count(), 1);
}

#[tokio::test]
async fn movies_keep_other_titles_once() {
    let server = server().await;
    Mock::given(path("/movie/438631"))
        .and(query_param("append_to_response", "release_dates,alternative_titles,images"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("movie_438631.json")))
        .mount(&server)
        .await;

    let dune = client(&server, "US").movie(ExternalId::Tmdb(438631)).await.unwrap();

    assert_eq!(dune.alternate_titles.iter().filter(|title| *title == "Dune: Part One").count(), 1);
    assert!(dune.alternate_titles.contains(&"Дюна".to_owned()));
}
