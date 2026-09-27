use std::sync::{Arc, Mutex};

use jiff::civil::date;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};
use yokoku_domain::{ExternalId, Live, MediaKind, Secret, SourceStatus};
use yokoku_library::ports::{MetadataError, MetadataProvider};
use yokoku_metadata::{MetadataSettings, TvdbClient, TvdbSettings};

const TOKEN: &str = "test-token";

fn client(server: &MockServer) -> TvdbClient {
    TvdbClient::new(Live::fixed(settings_at(&server.uri(), "api-key")))
}

fn settings_at(url: &str, api_key: &str) -> MetadataSettings {
    let tvdb =
        TvdbSettings { api_key: Some(Secret::new(api_key)), pin: Some(Secret::new("1234")), url: url.to_owned() };
    MetadataSettings { tvdb, ..MetadataSettings::default() }
}

fn ok(data: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "status": "success", "data": data }))
}

async fn mount_login(server: &MockServer, token: &str) {
    Mock::given(method("POST"))
        .and(path("/login"))
        .and(body_json(json!({ "apikey": "api-key", "pin": "1234" })))
        .respond_with(ok(json!({ "token": token })))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

fn episode(id: u64, season: u16, number: u16, name: Option<&str>, aired: Option<&str>) -> Value {
    json!({ "id": id, "seasonNumber": season, "number": number, "name": name, "aired": aired })
}

/// Serves Frieren with its episodes on two pages.
async fn mount_frieren(server: &MockServer) {
    Mock::given(path("/series/424536/extended"))
        .and(query_param("meta", "translations"))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .respond_with(ok(json!({
            "id": 424536, "name": "葬送のフリーレン", "year": "2023",
            "image": "https://artworks.thetvdb.com/banners/v4/series/424536/posters/1.jpg",
            "status": { "id": 1, "name": "Continuing", "recordType": "series", "keepUpdated": false },
            "aliases": [
                { "language": "eng", "name": "Frieren" },
                { "language": "jpn", "name": "Sousou no Frieren" },
                { "language": "eng", "name": "Frieren" },
                { "language": "eng", "name": "Frieren: Beyond Journey's End" },
            ],
            "translations": { "nameTranslations": [
                { "language": "jpn", "name": "葬送のフリーレン", "isPrimary": true, "isAlias": null },
                { "language": "eng", "name": "Frieren of the Funeral", "isPrimary": null, "isAlias": true },
                { "language": "eng", "name": "Frieren: Beyond Journey's End", "isPrimary": null, "isAlias": null },
            ]},
        })))
        .mount(server)
        .await;
    let page = |episodes: Vec<Value>, next: Option<&str>| {
        ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": { "series": { "id": 424536 }, "episodes": episodes },
            "links": { "prev": null, "self": "", "next": next, "total_items": 3, "page_size": 2 },
        }))
    };
    Mock::given(path("/series/424536/episodes/default/eng"))
        .and(query_param("page", "0"))
        .respond_with(page(
            vec![
                episode(9_000_001, 0, 1, Some("Recap"), None),
                episode(8_000_002, 1, 2, Some("It Didn't Have to Be Magic..."), Some("2023-09-29")),
            ],
            Some("https://api4.thetvdb.com/v4/series/424536/episodes/default/eng?page=1"),
        ))
        .mount(server)
        .await;
    Mock::given(path("/series/424536/episodes/default/eng"))
        .and(query_param("page", "1"))
        .respond_with(page(vec![episode(8_000_001, 1, 1, Some("The Journey's End"), Some("2023-09-29"))], None))
        .mount(server)
        .await;
}

#[tokio::test]
async fn search_finds_series_under_their_translated_names() {
    let server = MockServer::start().await;
    mount_login(&server, TOKEN).await;
    Mock::given(path("/search"))
        .and(query_param("query", "frieren"))
        .and(query_param("type", "series"))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .respond_with(ok(json!([
            {
                "tvdb_id": "424536", "name": "葬送のフリーレン", "year": "2023", "image_url": "https://example.test/1.jpg",
                "translations": { "eng": "Frieren: Beyond Journey's End", "jpn": "葬送のフリーレン" },
            },
            { "tvdb_id": "1", "name": "Frieren Shorts" },
            { "tvdb_id": "not-a-number", "name": "Broken" },
        ])))
        .mount(&server)
        .await;

    let results = client(&server).search("frieren").await.unwrap();

    let found: Vec<_> =
        results.iter().map(|r| (r.kind, r.source, r.title.as_str(), r.original_title.as_str(), r.year)).collect();
    assert_eq!(
        found,
        [
            (
                MediaKind::Series,
                ExternalId::Tvdb(424536),
                "Frieren: Beyond Journey's End",
                "葬送のフリーレン",
                Some(2023)
            ),
            (MediaKind::Series, ExternalId::Tvdb(1), "Frieren Shorts", "Frieren Shorts", None),
        ]
    );
}

#[tokio::test]
async fn series_gather_every_episode_page_into_seasons() {
    let server = MockServer::start().await;
    mount_login(&server, TOKEN).await;
    mount_frieren(&server).await;

    let frieren = client(&server).series(ExternalId::Tvdb(424536)).await.unwrap();

    assert_eq!(frieren.title, "Frieren: Beyond Journey's End");
    assert_eq!(frieren.original_title, "葬送のフリーレン");
    assert_eq!(frieren.year, Some(2023));
    assert_eq!(frieren.status, SourceStatus::Returning);
    assert_eq!(frieren.alternate_titles, ["Frieren", "Sousou no Frieren"]);
    assert_eq!(frieren.seasons.iter().map(|season| season.number).collect::<Vec<_>>(), [0, 1]);
    let first = &frieren.seasons[1].episodes[0];
    assert_eq!(
        (first.source_id, first.number, first.title.as_str(), first.air_date),
        (8_000_001, 1, "The Journey's End", Some(date(2023, 9, 29)))
    );
    assert_eq!(frieren.seasons[1].episodes.len(), 2);
}

#[tokio::test]
async fn one_login_serves_every_request() {
    let server = MockServer::start().await;
    Mock::given(path("/login")).respond_with(ok(json!({ "token": TOKEN }))).expect(1).mount(&server).await;
    mount_frieren(&server).await;
    let client = client(&server);

    client.series(ExternalId::Tvdb(424536)).await.unwrap();
    client.series(ExternalId::Tvdb(424536)).await.unwrap();
}

#[tokio::test]
async fn a_changed_api_key_logs_in_again() {
    let server = MockServer::start().await;
    let login = |api_key: &str| {
        Mock::given(path("/login"))
            .and(body_json(json!({ "apikey": api_key, "pin": "1234" })))
            .respond_with(ok(json!({ "token": TOKEN })))
            .expect(1)
    };
    login("api-key").mount(&server).await;
    login("new-key").mount(&server).await;
    mount_frieren(&server).await;
    let api_key = Arc::new(Mutex::new("api-key"));
    let client = TvdbClient::new(Live::new({
        let (server, api_key) = (server.uri(), api_key.clone());
        move || settings_at(&server, &api_key.lock().unwrap())
    }));

    client.series(ExternalId::Tvdb(424536)).await.unwrap();
    *api_key.lock().unwrap() = "new-key";
    client.series(ExternalId::Tvdb(424536)).await.unwrap();
}

#[tokio::test]
async fn a_refused_token_is_renewed_once() {
    let server = MockServer::start().await;
    mount_login(&server, "expired").await;
    mount_login(&server, TOKEN).await;
    Mock::given(path("/series/424536/extended"))
        .and(header("authorization", "Bearer expired"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    mount_frieren(&server).await;

    let frieren = client(&server).series(ExternalId::Tvdb(424536)).await.unwrap();

    assert_eq!(frieren.seasons.len(), 2);
}

#[tokio::test]
async fn a_refused_login_says_why() {
    let server = MockServer::start().await;
    let body = json!({ "status": "failure", "message": "InvalidAPIKey", "data": null });
    Mock::given(path("/login")).respond_with(ResponseTemplate::new(401).set_body_json(body)).mount(&server).await;

    let error = client(&server).series(ExternalId::Tvdb(424536)).await.unwrap_err();

    assert!(matches!(&error, MetadataError::Refused(reason) if reason.contains("InvalidAPIKey")), "{error:?}");
}

#[tokio::test]
async fn a_token_refused_twice_is_not_renewed_again() {
    let server = MockServer::start().await;
    Mock::given(path("/login")).respond_with(ok(json!({ "token": TOKEN }))).expect(2).mount(&server).await;
    Mock::given(path("/series/424536/extended"))
        .respond_with(ResponseTemplate::new(403))
        .expect(2)
        .mount(&server)
        .await;

    let error = client(&server).series(ExternalId::Tvdb(424536)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Refused(_)));
}

#[tokio::test]
async fn endless_episode_pages_are_invalid() {
    let server = MockServer::start().await;
    mount_login(&server, TOKEN).await;
    mount_frieren(&server).await;
    Mock::given(path("/series/424536/episodes/default/eng"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "episodes": [] }, "links": { "next": "more" },
        })))
        .with_priority(1)
        .mount(&server)
        .await;

    let error = client(&server).series(ExternalId::Tvdb(424536)).await.unwrap_err();

    assert!(matches!(error, MetadataError::Invalid(_)));
}

#[tokio::test]
async fn missing_series_are_not_found() {
    let server = MockServer::start().await;
    mount_login(&server, TOKEN).await;
    Mock::given(path("/series/404/extended")).respond_with(ResponseTemplate::new(404)).mount(&server).await;

    let error = client(&server).series(ExternalId::Tvdb(404)).await.unwrap_err();

    assert!(matches!(error, MetadataError::NotFound(ExternalId::Tvdb(404))));
}

#[tokio::test]
async fn movies_and_tmdb_ids_are_not_looked_up_on_tvdb() {
    let server = MockServer::start().await;
    let client = client(&server);

    let movie = client.movie(ExternalId::Tvdb(1)).await.unwrap_err();
    let series = client.series(ExternalId::Tmdb(209867)).await.unwrap_err();

    assert!(matches!(movie, MetadataError::Unavailable(_)));
    assert!(matches!(series, MetadataError::Unavailable(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}
