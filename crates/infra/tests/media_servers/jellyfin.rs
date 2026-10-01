use std::path::PathBuf;

use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};
use yokoku_core::integrations::ports::{MediaServer, MediaServerError, Played, PlayedItem};
use yokoku_domain::{EpisodeSpan, ExternalId, Live, Secret};
use yokoku_infra::media_servers::{JellyfinClient, JellyfinSettings};
use yokoku_test_support::jellyfin::system_info;

const AUTHORIZATION: &str = "MediaBrowser Token=\"secret\"";

fn client(url: impl Into<String>, api_key: &str) -> JellyfinClient {
    JellyfinClient::new(Live::fixed(JellyfinSettings {
        url: Some(url.into()),
        api_key: Some(Secret::new(api_key)),
        user: None,
    }))
}

#[tokio::test]
async fn refreshes_the_library_with_the_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Library/Refresh"))
        .and(header("Authorization", AUTHORIZATION))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    client(format!("{}/", server.uri()), "secret").refresh_library().await.unwrap();
}

#[tokio::test]
async fn reports_the_server_version() {
    let server = MockServer::start().await;
    system_info("secret", "10.10.7").mount(&server).await;

    assert_eq!(client(server.uri(), "secret").version().await.unwrap(), "Jellyfin 10.10.7");
}

#[tokio::test]
async fn a_rejected_key_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(403)).mount(&server).await;

    let error = client(server.uri(), "not an admin").refresh_library().await.unwrap_err();

    assert!(matches!(error, MediaServerError::Refused(_)), "{error}");
}

#[tokio::test]
async fn an_unreachable_server_is_unavailable() {
    let error = client("http://127.0.0.1:9", "secret").refresh_library().await.unwrap_err();

    assert!(matches!(error, MediaServerError::Unavailable(_)), "{error}");
}

#[tokio::test]
async fn a_server_without_a_url_is_not_configured() {
    let unset = JellyfinClient::new(Live::fixed(JellyfinSettings::default()));

    assert!(matches!(unset.refresh_library().await.unwrap_err(), MediaServerError::NotConfigured));
}

fn client_for(server: &MockServer, user: Option<&str>) -> JellyfinClient {
    JellyfinClient::new(Live::fixed(JellyfinSettings {
        url: Some(server.uri()),
        api_key: Some(Secret::new("secret")),
        user: user.map(Into::into),
    }))
}

async fn mount_users(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/Users"))
        .and(header("Authorization", AUTHORIZATION))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "Id": "u-other", "Name": "guest" },
            { "Id": "u-1", "Name": "Admin" },
        ])))
        .mount(server)
        .await;
}

fn items(types: &str, start: usize, total: usize, items: Value) -> Mock {
    Mock::given(method("GET"))
        .and(path("/Items"))
        .and(query_param("userId", "u-1"))
        .and(query_param("includeItemTypes", types))
        .and(query_param("startIndex", start.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "Items": items, "TotalRecordCount": total })))
}

#[tokio::test]
async fn played_items_carry_their_path_episodes_and_provider_ids() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let played = json!([
        {
            "Id": "e1", "Type": "Episode", "SeriesId": "s1", "ParentIndexNumber": 1, "IndexNumber": 4,
            "IndexNumberEnd": 5, "ProviderIds": { "Tvdb": "8891224", "Imdb": "tt13411248" },
            "Path": "/media/tv/Severance/Season 01/S01E04-E05.mkv",
            "UserData": { "Played": true, "LastPlayedDate": "2026-10-01T20:50:55.2577389Z" },
        },
        {
            "Id": "m1", "Type": "Movie", "ProviderIds": { "Tmdb": "329865", "Imdb": "tt2543164" },
            "Path": "/media/movies/Arrival (2016)/Arrival (2016).mkv", "UserData": { "Played": true },
        },
        { "Id": "e2", "Type": "Episode", "SeriesId": "s1", "Path": "/media/tv/Severance/extra.mkv" },
        { "Id": "f1", "Type": "Episode", "SeriesId": "s1", "ParentIndexNumber": 1, "IndexNumber": 1 },
    ]);
    let series = json!([{ "Id": "s1", "Type": "Series", "ProviderIds": { "Tvdb": "371980", "Tmdb": "95396" } }]);
    items("Episode,Movie", 0, 4, played).mount(&server).await;
    items("Series", 0, 1, series).mount(&server).await;

    let played = client_for(&server, Some("admin")).played().await.unwrap();

    let series = vec![ExternalId::Tmdb(95396), ExternalId::Tvdb(371980)];
    assert_eq!(
        played,
        [
            Played {
                path: "/media/tv/Severance/Season 01/S01E04-E05.mkv".into(),
                item: Some(PlayedItem::Episodes { series, span: EpisodeSpan::new(1, 4, 5).unwrap() }),
                at: Some("2026-10-01T20:50:55.2577389Z".parse().unwrap()),
            },
            Played {
                path: "/media/movies/Arrival (2016)/Arrival (2016).mkv".into(),
                item: Some(PlayedItem::Movie(vec![ExternalId::Tmdb(329865)])),
                at: None,
            },
            Played { path: "/media/tv/Severance/extra.mkv".into(), item: None, at: None },
        ]
    );
}

#[tokio::test]
async fn played_items_are_read_a_page_at_a_time() {
    let server = MockServer::start().await;
    mount_users(&server).await;
    let movie = |n: u32| json!([{ "Id": format!("m{n}"), "Type": "Movie", "Path": format!("/movies/{n}.mkv") }]);
    items("Episode,Movie", 0, 2, movie(1)).expect(1).mount(&server).await;
    items("Episode,Movie", 1, 2, movie(2)).expect(1).mount(&server).await;
    items("Series", 0, 0, json!([])).mount(&server).await;

    let played = client_for(&server, Some("admin")).played().await.unwrap();

    let paths: Vec<_> = played.into_iter().map(|played| played.path).collect();
    assert_eq!(paths, [PathBuf::from("/movies/1.mkv"), PathBuf::from("/movies/2.mkv")]);
}

#[tokio::test]
async fn an_unknown_user_is_refused() {
    let server = MockServer::start().await;
    mount_users(&server).await;

    let error = client_for(&server, Some("nobody")).played().await.unwrap_err();

    assert!(matches!(error, MediaServerError::Refused(_)), "{error}");
}

#[tokio::test]
async fn played_items_without_a_user_are_not_configured() {
    let server = MockServer::start().await;

    let error = client_for(&server, None).played().await.unwrap_err();

    assert!(matches!(error, MediaServerError::NotConfigured), "{error}");
}
