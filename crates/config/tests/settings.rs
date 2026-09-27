use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};
use yokoku_config::Settings;
use yokoku_domain::{SettingsStore, StorageError};
use yokoku_media::ImportMode;

#[derive(Default)]
struct MemoryStore(Mutex<Vec<(String, Value)>>);

#[async_trait]
impl SettingsStore for MemoryStore {
    async fn settings(&self) -> Result<Vec<(String, Value)>, StorageError> {
        Ok(self.0.lock().unwrap().clone())
    }

    async fn set_setting(&self, key: &str, value: &Value) -> Result<(), StorageError> {
        let mut stored = self.0.lock().unwrap();
        stored.retain(|(stored, _)| stored != key);
        stored.push((key.to_owned(), value.clone()));
        Ok(())
    }

    async fn remove_setting(&self, key: &str) -> Result<bool, StorageError> {
        let mut stored = self.0.lock().unwrap();
        let before = stored.len();
        stored.retain(|(stored, _)| stored != key);
        Ok(stored.len() < before)
    }
}

async fn open(store: &Arc<MemoryStore>) -> Settings {
    Settings::open(None, store.clone()).await.unwrap()
}

#[tokio::test]
async fn stored_settings_go_over_the_defaults() {
    let store = Arc::new(MemoryStore::default());
    store.set_setting("import.mode", &json!("copy")).await.unwrap();
    store.set_setting("calendar.days", &json!(14)).await.unwrap();
    store.set_setting("downloads.pick_up_labels", &json!(["tv", "anime"])).await.unwrap();

    let config = open(&store).await.current();

    assert_eq!(config.import.mode, ImportMode::Copy);
    assert_eq!(config.calendar.days, Some(14));
    assert_eq!(config.downloads.pick_up_labels, ["tv", "anime"]);
}

#[tokio::test]
async fn a_live_value_follows_a_reload() {
    let store = Arc::new(MemoryStore::default());
    let settings = open(&store).await;
    let mode = settings.live(|config| config.import.mode);

    store.set_setting("import.mode", &json!("move")).await.unwrap();
    let before = mode.current();
    settings.reload().await.unwrap();

    assert_eq!((before, mode.current()), (ImportMode::HardLink, ImportMode::Move));
}

#[tokio::test]
async fn a_reload_that_fails_keeps_the_settings_in_effect() {
    let store = Arc::new(MemoryStore::default());
    store.set_setting("import.mode", &json!("copy")).await.unwrap();
    let settings = open(&store).await;

    store.set_setting("serve.scan_library", &json!("every day")).await.unwrap();
    let error = settings.reload().await.unwrap_err();

    assert!(format!("{error:#}").contains("Invalid schedule"), "{error:#}");
    assert_eq!(settings.current().import.mode, ImportMode::Copy);
}
