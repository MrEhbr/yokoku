//! The configuration as the web's Settings page reaches it.

use async_trait::async_trait;
use serde_json::Value;
use yokoku_core::{
    downloads::{
        DownloadError,
        ports::{DownloadClient, Indexer},
    },
    integrations::ports::MediaServer,
    notifications::ports::{Notification, Notifier, Priority},
};
use yokoku_domain::{Live, StorageError};
use yokoku_infra::{
    download_clients::TransmissionClient,
    indexers::JackettClient,
    media_servers::JellyfinClient,
    notifiers::{NtfyClient, TelegramClient},
};
use yokoku_web::{Connection, ConnectionTest, Field, SettingsAccess};

use crate::config::{Config, Settings};

#[async_trait]
impl SettingsAccess for Settings {
    fn fields(&self) -> Vec<Field> {
        Config::fields()
    }

    fn value(&self, key: &str) -> Option<Value> {
        self.current().value(key).ok()
    }

    fn set_by_env(&self, key: &str) -> bool {
        Config::set_by_env(key)
    }

    async fn stored_keys(&self) -> Result<Vec<String>, String> {
        Settings::stored_keys(self).await.map_err(message)
    }

    async fn set(&self, key: &str, value: Value) -> Result<(), String> {
        Settings::set(self, key, value).await.map_err(message)
    }

    async fn unset(&self, key: &str) -> Result<(), String> {
        Settings::unset(self, key).await.map(drop).map_err(message)
    }
}

/// Reaches Transmission, Jellyfin or Jackett, or sends a test notification through ntfy or
/// Telegram, with settings that are not stored yet.
#[async_trait]
impl ConnectionTest for Settings {
    async fn test(&self, connection: Connection, changes: Vec<(String, Option<Value>)>) -> Result<String, String> {
        let config = self.preview(&changes).await.map_err(message)?;
        let test = Notification {
            title: "Yokoku".to_owned(),
            message: "Notifications from Yokoku arrive here.".to_owned(),
            priority: Priority::Normal,
        };
        match connection {
            Connection::Transmission => TransmissionClient::new(Live::fixed(config.transmission.clone()))
                .version()
                .await
                .map_err(|error| DownloadError::from(error).to_string()),
            Connection::Jellyfin if config.jellyfin.url.is_none() => Err("Set the Jellyfin address first".to_owned()),
            Connection::Jellyfin => JellyfinClient::new(Live::fixed(config.jellyfin.clone()))
                .version()
                .await
                .map_err(|error| error.to_string()),
            Connection::Jackett if config.jackett.url.is_none() => Err("Set the Jackett address first".to_owned()),
            Connection::Jackett => JackettClient::new(Live::fixed(config.jackett.clone()))
                .version()
                .await
                .map_err(|error| error.to_string()),
            Connection::Ntfy => {
                let ntfy = &config.notifications.ntfy;
                let Some(topic) = ntfy.topic.clone() else {
                    return Err("Set the topic first".to_owned());
                };
                NtfyClient::new(Live::fixed(ntfy.clone()))
                    .send(&test)
                    .await
                    .map(|()| topic)
                    .map_err(|error| error.to_string())
            },
            Connection::Telegram => {
                let telegram = &config.notifications.telegram;
                if telegram.token.is_none() {
                    return Err("Set the bot token first".to_owned());
                }
                let Some(chat) = telegram.chat_id.clone() else {
                    return Err("Set the chat first".to_owned());
                };
                TelegramClient::new(Live::fixed(telegram.clone()))
                    .send(&test)
                    .await
                    .map(|()| chat)
                    .map_err(|error| error.to_string())
            },
        }
    }
}

/// Why a value does not load, or a generic message for a storage failure, which goes to the log.
fn message(error: anyhow::Error) -> String {
    if error.chain().any(|cause| cause.is::<StorageError>()) {
        tracing::error!(error = format!("{error:#}"), "storing a setting failed");
        return "The settings could not be stored; the server log has the cause".to_owned();
    }
    format!("{error:#}")
}
