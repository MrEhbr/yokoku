use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};
use yokoku_domain::{Live, Secret};
use yokoku_integrations::ports::{MediaServer, MediaServerError};
use yokoku_media_servers::{JellyfinClient, JellyfinSettings};
use yokoku_test_support::jellyfin::system_info;

const AUTHORIZATION: &str = "MediaBrowser Token=\"secret\"";

fn client(url: impl Into<String>, api_key: &str) -> JellyfinClient {
    JellyfinClient::new(Live::fixed(JellyfinSettings { url: Some(url.into()), api_key: Some(Secret::new(api_key)) }))
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
