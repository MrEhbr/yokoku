use rstest::rstest;
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};
use yokoku_core::notifications::ports::{Notification, Notifier, NotifyError, Priority};
use yokoku_domain::{Live, Secret};
use yokoku_infra::notifiers::{TelegramClient, TelegramSettings};

const TOKEN: &str = "123456:dev";
const CHAT: &str = "-1001234567890";

fn settings(url: String) -> TelegramSettings {
    TelegramSettings { url, token: Some(Secret::new(TOKEN)), chat_id: Some(CHAT.to_owned()) }
}

fn client(server: &MockServer) -> TelegramClient {
    TelegramClient::new(Live::fixed(settings(format!("{}/", server.uri()))))
}

fn notification(title: &str, message: &str, priority: Priority) -> Notification {
    Notification { title: title.to_owned(), message: message.to_owned(), priority }
}

fn sent() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({"ok": true, "result": {}}))
}

#[rstest]
#[case::normal_is_silent(Priority::Normal, true)]
#[case::high_rings(Priority::High, false)]
#[tokio::test]
async fn a_notification_is_sent_to_the_chat_with_its_title_in_bold(#[case] priority: Priority, #[case] silent: bool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .and(body_json(json!({
            "chat_id": CHAT,
            "text": "<b>Frieren (2023)</b>\nImported 2 files\nS01E05\nS01E06",
            "parse_mode": "HTML",
            "disable_notification": silent,
        })))
        .respond_with(sent())
        .expect(1)
        .mount(&server)
        .await;

    client(&server).send(&notification("Frieren (2023)", "Imported 2 files\nS01E05\nS01E06", priority)).await.unwrap();
}

#[tokio::test]
async fn markup_in_the_notification_is_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_json(json!({
            "chat_id": CHAT,
            "text": "<b>Tom &amp; Jerry</b>\n&lt;i&gt;Tom.mkv&lt;/i&gt;: no space",
            "parse_mode": "HTML",
            "disable_notification": true,
        })))
        .respond_with(sent())
        .expect(1)
        .mount(&server)
        .await;

    client(&server).send(&notification("Tom & Jerry", "<i>Tom.mkv</i>: no space", Priority::Normal)).await.unwrap();
}

#[rstest]
#[case::with_telegram_description(
    ResponseTemplate::new(400).set_body_json(json!({
        "ok": false,
        "error_code": 400,
        "description": "Bad Request: chat not found",
    })),
    "HTTP 400 Bad Request: Bad Request: chat not found"
)]
#[case::without_a_body(ResponseTemplate::new(502), "HTTP 502 Bad Gateway")]
#[tokio::test]
async fn a_refusal_carries_the_status_and_telegram_description(
    #[case] response: ResponseTemplate,
    #[case] expected: &str,
) {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(response).mount(&server).await;

    let error = client(&server).send(&notification("Frieren", "Imported S01E05", Priority::Normal)).await.unwrap_err();

    assert!(matches!(&error, NotifyError::Refused(reason) if reason == expected), "{error:?}");
}

#[rstest]
#[case::without_a_token(TelegramSettings { token: None, ..settings(String::new()) })]
#[case::without_a_chat(TelegramSettings { chat_id: None, ..settings(String::new()) })]
#[tokio::test]
async fn nothing_is_sent_without_a_token_and_chat(#[case] unset: TelegramSettings) {
    let server = MockServer::start().await;
    let client = TelegramClient::new(Live::fixed(TelegramSettings { url: server.uri(), ..unset }));

    let error = client.send(&notification("Frieren", "Imported S01E05", Priority::Normal)).await.unwrap_err();

    assert!(matches!(error, NotifyError::NotConfigured), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unreachable_server_is_unavailable_without_the_token_in_the_error() {
    let client = TelegramClient::new(Live::fixed(settings("http://127.0.0.1:9".to_owned())));

    let error = client.send(&notification("Frieren", "Imported S01E05", Priority::Normal)).await.unwrap_err();

    assert!(matches!(error, NotifyError::Unavailable(_)), "{error:?}");
    let chain: Vec<String> = std::iter::successors(Some(&error as &dyn std::error::Error), |cause| cause.source())
        .map(|cause| cause.to_string())
        .collect();
    assert!(!chain.iter().any(|cause| cause.contains(TOKEN)), "{chain:?}");
}
