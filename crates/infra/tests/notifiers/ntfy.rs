use rstest::rstest;
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};
use yokoku_core::notifications::ports::{Notification, Notifier, NotifyError, Priority};
use yokoku_domain::{Live, Secret};
use yokoku_infra::notifiers::{NtfyClient, NtfySettings};

const TOPIC: &str = "yokoku-dev";

fn client(server: &MockServer, token: Option<&str>) -> NtfyClient {
    NtfyClient::new(Live::fixed(NtfySettings {
        url: format!("{}/", server.uri()),
        topic: Some(TOPIC.to_owned()),
        token: token.map(Secret::new),
    }))
}

fn imported(priority: Priority) -> Notification {
    Notification { title: "Frieren".to_owned(), message: "Imported S01E05".to_owned(), priority }
}

#[rstest]
#[case::normal(Priority::Normal, 3)]
#[case::high(Priority::High, 4)]
#[tokio::test]
async fn a_notification_is_published_to_the_topic_from_the_server_root(
    #[case] priority: Priority,
    #[case] expected: u8,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("authorization", "Bearer tk_dev"))
        .and(body_json(json!({
            "topic": TOPIC,
            "title": "Frieren",
            "message": "Imported S01E05",
            "priority": expected,
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    client(&server, Some("tk_dev")).send(&imported(priority)).await.unwrap();
}

#[tokio::test]
async fn a_topic_without_a_token_is_published_without_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&server).await;

    client(&server, None).send(&imported(Priority::Normal)).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[rstest]
#[case::with_ntfy_error(
    ResponseTemplate::new(403).set_body_json(json!({"code": 40301, "http": 403, "error": "forbidden"})),
    "HTTP 403 Forbidden: forbidden"
)]
#[case::without_a_body(ResponseTemplate::new(502), "HTTP 502 Bad Gateway")]
#[tokio::test]
async fn a_refusal_carries_the_status_and_ntfy_error(#[case] response: ResponseTemplate, #[case] expected: &str) {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(response).mount(&server).await;

    let error = client(&server, None).send(&imported(Priority::Normal)).await.unwrap_err();

    assert!(matches!(&error, NotifyError::Refused(reason) if reason == expected), "{error:?}");
}

#[tokio::test]
async fn nothing_is_published_without_a_topic() {
    let server = MockServer::start().await;
    let client = NtfyClient::new(Live::fixed(NtfySettings { url: server.uri(), ..NtfySettings::default() }));

    let error = client.send(&imported(Priority::Normal)).await.unwrap_err();

    assert!(matches!(error, NotifyError::NotConfigured), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unreachable_server_is_unavailable() {
    let settings = NtfySettings { url: "http://127.0.0.1:9".to_owned(), topic: Some(TOPIC.to_owned()), token: None };

    let error = NtfyClient::new(Live::fixed(settings)).send(&imported(Priority::Normal)).await.unwrap_err();

    assert!(matches!(error, NotifyError::Unavailable(_)), "{error:?}");
}
