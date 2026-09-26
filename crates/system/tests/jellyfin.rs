use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};
use yokoku_integrations::ports::{MediaServer, MediaServerError};
use yokoku_system::JellyfinClient;

const AUTHORIZATION: &str = "MediaBrowser Token=\"secret\"";

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

    JellyfinClient::new(format!("{}/", server.uri()), "secret").refresh_library().await.unwrap();
}

#[tokio::test]
async fn reports_the_server_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/System/Info"))
        .and(header("Authorization", AUTHORIZATION))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ServerName": "media", "Version": "10.10.7" })))
        .mount(&server)
        .await;

    assert_eq!(JellyfinClient::new(server.uri(), "secret").version().await.unwrap(), "Jellyfin 10.10.7");
}

#[tokio::test]
async fn a_rejected_key_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(403)).mount(&server).await;

    let error = JellyfinClient::new(server.uri(), "not an admin").refresh_library().await.unwrap_err();

    assert!(matches!(error, MediaServerError::Refused(_)), "{error}");
}

#[tokio::test]
async fn an_unreachable_server_is_unavailable() {
    let error = JellyfinClient::new("http://127.0.0.1:9", "secret").refresh_library().await.unwrap_err();

    assert!(matches!(error, MediaServerError::Unavailable(_)), "{error}");
}
