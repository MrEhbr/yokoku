use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};
use yokoku_core::media::ImportMode;
use yokoku_domain::{SettingsStore, StorageError};

use crate::config::Settings;

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
    store.set_setting("transmission.username", &json!("yokoku")).await.unwrap();
    store.set_setting("downloads.pick_up_labels", &json!(["tv", "anime"])).await.unwrap();

    let config = open(&store).await.current();

    assert_eq!(config.import.mode, ImportMode::Copy);
    assert_eq!(config.transmission.username.as_deref(), Some("yokoku"));
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
async fn a_preview_applies_changes_without_storing_them() {
    let store = Arc::new(MemoryStore::default());
    store.set_setting("import.mode", &json!("copy")).await.unwrap();
    store.set_setting("transmission.username", &json!("yokoku")).await.unwrap();
    let settings = open(&store).await;

    let preview = settings
        .preview(&[("import.mode".into(), Some(json!("move"))), ("transmission.username".into(), None)])
        .await
        .unwrap();
    let invalid = settings.preview(&[("import.mode".into(), Some(json!("teleport")))]).await;

    assert_eq!((preview.import.mode, preview.transmission.username), (ImportMode::Move, None));
    assert!(invalid.is_err());
    assert_eq!(settings.current().import.mode, ImportMode::Copy);
    assert_eq!(settings.stored_keys().await.unwrap(), ["import.mode", "transmission.username"]);
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

#[tokio::test]
async fn the_web_address_defaults_to_localhost() {
    let store = Arc::new(MemoryStore::default());

    let address = open(&store).await.current().web.address();

    assert_eq!(address.to_string(), "127.0.0.1:8080");
}

#[tokio::test]
async fn setting_a_value_stores_and_applies_it() {
    let store = Arc::new(MemoryStore::default());
    let settings = open(&store).await;

    settings.set("import.mode", json!("copy")).await.unwrap();

    assert_eq!(settings.current().import.mode, ImportMode::Copy);
    assert_eq!(settings.stored_keys().await.unwrap(), ["import.mode"]);
}

#[tokio::test]
async fn a_value_that_does_not_load_is_refused_and_not_stored() {
    let store = Arc::new(MemoryStore::default());
    let settings = open(&store).await;

    let wrong_value = settings.set("import.mode", json!("teleport")).await.unwrap_err();
    let unknown_key = settings.set("import.speed", json!(1)).await.unwrap_err();
    let section = settings.set("naming", json!("x")).await.unwrap_err();
    let database = settings.set("database.path", json!("/tmp/other.db")).await.unwrap_err();
    let port = settings.set("web.port", json!(9000)).await.unwrap_err();

    assert!(format!("{wrong_value:#}").contains("teleport"), "{wrong_value:#}");
    assert!(format!("{unknown_key:#}").contains("not a setting"), "{unknown_key:#}");
    assert!(format!("{section:#}").contains("naming is not a setting"), "{section:#}");
    for at_start in [database, port] {
        assert!(format!("{at_start:#}").contains("read when the service starts"), "{at_start:#}");
    }
    assert_eq!(settings.stored_keys().await.unwrap(), Vec::<String>::new());
}

#[tokio::test]
async fn unsetting_a_value_brings_back_the_default() {
    let store = Arc::new(MemoryStore::default());
    let settings = open(&store).await;
    settings.set("import.mode", json!("move")).await.unwrap();

    let removed = settings.unset("import.mode").await.unwrap();

    assert!(removed);
    assert_eq!(settings.current().import.mode, ImportMode::HardLink);
}
