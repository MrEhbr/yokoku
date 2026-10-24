use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;
use yokoku_core::notifications::ports::{Notification, Notifier, NotifyError, Priority};
use yokoku_domain::{Live, Secret};

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NtfySettings {
    /// Server address: the public `https://ntfy.sh` or a self-hosted one.
    pub url: String,
    /// Notifications are off while unset.
    pub topic: Option<String>,
    /// An access token, for a topic that needs one.
    pub token: Option<Secret>,
}

impl Default for NtfySettings {
    fn default() -> Self {
        Self { url: "https://ntfy.sh".to_owned(), topic: None, token: None }
    }
}

/// Publishes notifications to a topic on an ntfy server.
pub struct NtfyClient {
    http: reqwest::Client,
    settings: Live<NtfySettings>,
}

impl NtfyClient {
    pub fn new(settings: Live<NtfySettings>) -> Self {
        crate::tls::install_crypto_provider();
        let http = reqwest::Client::builder().timeout(TIMEOUT).build().expect("TLS backend initializes");
        Self { http, settings }
    }
}

/// A message as ntfy publishes it from JSON, posted to the server's root.
#[derive(Serialize)]
struct Message<'a> {
    topic: &'a str,
    title: &'a str,
    message: &'a str,
    /// 1 to 5; ntfy's default is 3.
    priority: u8,
}

#[derive(Deserialize)]
struct Refusal {
    error: String,
}

#[async_trait]
impl Notifier for NtfyClient {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError> {
        let settings = self.settings.current();
        let topic = settings.topic.as_deref().ok_or(NotifyError::NotConfigured)?;
        let message = Message {
            topic,
            title: &notification.title,
            message: &notification.message,
            priority: match notification.priority {
                Priority::Normal => 3,
                Priority::High => 4,
            },
        };
        let mut request = self.http.post(format!("{}/", settings.url.trim_end_matches('/'))).json(&message);
        if let Some(token) = &settings.token {
            request = request.bearer_auth(token.expose());
        }
        let response = request.send().await.map_err(|error| NotifyError::Unavailable(error.into()))?;
        let status = response.status();
        debug!(status = status.as_u16(), "ntfy publish");
        if status.is_success() {
            return Ok(());
        }
        let reason = match response.json::<Refusal>().await {
            Ok(refusal) => format!("HTTP {status}: {}", refusal.error),
            Err(_) => format!("HTTP {status}"),
        };
        Err(NotifyError::Refused(reason))
    }
}
