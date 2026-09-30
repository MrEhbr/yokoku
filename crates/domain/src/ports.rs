use std::error::Error;

use async_trait::async_trait;
use jiff::Zoned;
use serde_json::Value;

pub trait Clock: Send + Sync {
    /// The current time in the user's time zone.
    fn now(&self) -> Zoned;
}

/// Settings by dotted key, such as `import.mode`, stored over the config file.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// Ordered by key.
    async fn settings(&self) -> Result<Vec<(String, Value)>, StorageError>;

    async fn set_setting(&self, key: &str, value: &Value) -> Result<(), StorageError>;

    /// `false` when nothing was stored under `key`.
    async fn remove_setting(&self, key: &str) -> Result<bool, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("the item was changed or removed at the same time")]
    Conflict,
    #[error(transparent)]
    Other(Box<dyn Error + Send + Sync>),
}

impl StorageError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::Other(source.into())
    }
}
