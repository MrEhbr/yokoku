//! Notifier adapters: ntfy, through its JSON publishing, and Telegram, through a bot.

mod ntfy;
mod telegram;

use async_trait::async_trait;
pub use ntfy::{NtfyClient, NtfySettings};
use serde::{Deserialize, Serialize};
pub use telegram::{TelegramClient, TelegramSettings};
use yokoku_core::notifications::ports::{Notification, Notifier, NotifyError};
use yokoku_domain::Live;

/// The service notifications go through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationService {
    #[default]
    Ntfy,
    Telegram,
}

/// Sends through the service chosen at the time of each send.
pub struct ChosenNotifier {
    service: Live<NotificationService>,
    ntfy: NtfyClient,
    telegram: TelegramClient,
}

impl ChosenNotifier {
    pub fn new(service: Live<NotificationService>, ntfy: NtfyClient, telegram: TelegramClient) -> Self {
        Self { service, ntfy, telegram }
    }
}

#[async_trait]
impl Notifier for ChosenNotifier {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError> {
        match self.service.current() {
            NotificationService::Ntfy => self.ntfy.send(notification).await,
            NotificationService::Telegram => self.telegram.send(notification).await,
        }
    }
}
