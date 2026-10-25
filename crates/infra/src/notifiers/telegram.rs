use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;
use yokoku_core::notifications::ports::{Notification, Notifier, NotifyError, Priority};
use yokoku_domain::{Live, Secret};

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramSettings {
    /// The Bot API's address.
    pub url: String,
    /// The bot's token from BotFather; notifications are off while unset.
    pub token: Option<Secret>,
    /// The chat the bot writes to: a numeric id, or `@name` for a public channel. Notifications are
    /// off while unset.
    pub chat_id: Option<String>,
}

impl Default for TelegramSettings {
    fn default() -> Self {
        Self { url: "https://api.telegram.org".to_owned(), token: None, chat_id: None }
    }
}

/// Sends notifications to a chat through a Telegram bot.
pub struct TelegramClient {
    http: reqwest::Client,
    settings: Live<TelegramSettings>,
}

impl TelegramClient {
    pub fn new(settings: Live<TelegramSettings>) -> Self {
        crate::tls::install_crypto_provider();
        let http = reqwest::Client::builder().timeout(TIMEOUT).build().expect("TLS backend initializes");
        Self { http, settings }
    }
}

/// The Bot API's `sendMessage` parameters.
#[derive(Serialize)]
struct Message<'a> {
    chat_id: &'a str,
    text: String,
    parse_mode: &'static str,
    /// Delivered without a sound.
    disable_notification: bool,
}

#[derive(Deserialize)]
struct Refusal {
    description: String,
}

/// Escapes the characters Telegram's HTML parse mode reserves.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[async_trait]
impl Notifier for TelegramClient {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError> {
        let settings = self.settings.current();
        let (Some(token), Some(chat_id)) = (&settings.token, settings.chat_id.as_deref()) else {
            return Err(NotifyError::NotConfigured);
        };
        let message = Message {
            chat_id,
            text: format!("<b>{}</b>\n{}", escape(&notification.title), escape(&notification.message)),
            parse_mode: "HTML",
            disable_notification: notification.priority == Priority::Normal,
        };
        let url = format!("{}/bot{}/sendMessage", settings.url.trim_end_matches('/'), token.expose());
        let response = self
            .http
            .post(url)
            .json(&message)
            .send()
            .await
            .map_err(|error| NotifyError::Unavailable(error.without_url().into()))?;
        let status = response.status();
        debug!(status = status.as_u16(), "telegram send");
        if status.is_success() {
            return Ok(());
        }
        let reason = match response.json::<Refusal>().await {
            Ok(refusal) => format!("HTTP {status}: {}", refusal.description),
            Err(_) => format!("HTTP {status}"),
        };
        Err(NotifyError::Refused(reason))
    }
}
