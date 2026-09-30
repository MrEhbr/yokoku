use std::env;

use async_trait::async_trait;
use serde_json::Value;
use yokoku_config::Settings;
use yokoku_domain::{Live, StorageError};
use yokoku_download_clients::TransmissionClient;
use yokoku_downloads::{DownloadError, ports::DownloadClient};
use yokoku_integrations::ports::MediaServer;
use yokoku_media_servers::JellyfinClient;
use yokoku_web::{Connection, SettingsAccess};

/// The configuration, as the web Settings page reaches it.
pub struct WebSettings {
    settings: Settings,
}

impl WebSettings {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl SettingsAccess for WebSettings {
    fn value(&self, key: &str) -> Option<Value> {
        let json = self.settings.current().setting(key).ok()?;
        serde_json::from_str(&json).ok()
    }

    fn set_by_env(&self, key: &str) -> bool {
        let variable = format!("APP__{}", key.to_uppercase().replace('.', "__"));
        env::var_os(&variable).is_some() || env::var_os(format!("{variable}__FILE")).is_some()
    }

    async fn stored_keys(&self) -> Result<Vec<String>, String> {
        self.settings.stored_keys().await.map_err(message)
    }

    async fn set(&self, key: &str, value: Value) -> Result<(), String> {
        self.settings.set(key, value).await.map_err(message)
    }

    async fn unset(&self, key: &str) -> Result<(), String> {
        self.settings.unset(key).await.map(drop).map_err(message)
    }

    async fn test(&self, connection: Connection, changes: Vec<(String, Option<Value>)>) -> Result<String, String> {
        let config = self.settings.preview(&changes).await.map_err(message)?;
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
