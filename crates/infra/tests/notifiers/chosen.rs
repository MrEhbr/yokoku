use rstest::rstest;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
use yokoku_core::notifications::ports::{Notification, Notifier, Priority};
use yokoku_domain::{Live, Secret};
use yokoku_infra::notifiers::{
    ChosenNotifier, NotificationService, NtfyClient, NtfySettings, TelegramClient, TelegramSettings,
};

#[rstest]
#[case::ntfy(NotificationService::Ntfy, 1, 0)]
#[case::telegram(NotificationService::Telegram, 0, 1)]
#[tokio::test]
async fn a_notification_goes_through_the_chosen_service_only(
    #[case] service: NotificationService,
    #[case] to_ntfy: usize,
    #[case] to_telegram: usize,
) {
    let (ntfy, telegram) = (MockServer::start().await, MockServer::start().await);
    for server in [&ntfy, &telegram] {
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(server).await;
    }
    let notifier = ChosenNotifier::new(
        Live::fixed(service),
        NtfyClient::new(Live::fixed(NtfySettings {
            url: ntfy.uri(),
            topic: Some("yokoku-dev".to_owned()),
            token: None,
        })),
        TelegramClient::new(Live::fixed(TelegramSettings {
            url: telegram.uri(),
            token: Some(Secret::new("123456:dev")),
            chat_id: Some("42".to_owned()),
        })),
    );

    let notification =
        Notification { title: "Frieren".to_owned(), message: "Imported S01E05".to_owned(), priority: Priority::Normal };
    notifier.send(&notification).await.unwrap();

    assert_eq!(ntfy.received_requests().await.unwrap().len(), to_ntfy);
    assert_eq!(telegram.received_requests().await.unwrap().len(), to_telegram);
}
