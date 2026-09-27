use async_trait::async_trait;
use serde_json::Value;

use crate::StorageError;

/// Settings by dotted key, such as `import.mode`, stored over the config file.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// Ordered by key.
    async fn settings(&self) -> Result<Vec<(String, Value)>, StorageError>;

    async fn set_setting(&self, key: &str, value: &Value) -> Result<(), StorageError>;

    /// `false` when nothing was stored under `key`.
    async fn remove_setting(&self, key: &str) -> Result<bool, StorageError>;
}
