use std::{env, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use yokoku_config::Settings;
use yokoku_domain::StorageError;
use yokoku_integrations::Rescans;
use yokoku_web::SettingsAccess;

/// The configuration and the Jellyfin connection, as the web Settings page reaches them.
pub struct WebSettings {
    settings: Settings,
    rescans: Arc<Rescans>,
}

impl WebSettings {
    pub fn new(settings: Settings, rescans: Arc<Rescans>) -> Self {
        Self { settings, rescans }
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

    async fn test_jellyfin(&self) -> Result<String, String> {
        if self.settings.current().jellyfin.url.is_none() {
            return Err("Set the Jellyfin address first".to_owned());
        }
        self.rescans.test_connection().await.map_err(|error| error.to_string())
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
