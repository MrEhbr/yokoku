use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use yokoku_core::events::{Handler, HandlerError, SettingsChanged};
use yokoku_domain::{Live, SettingsStore};

use crate::Config;

/// The configuration in effect: the config file, then the stored settings, then the environment.
#[derive(Clone)]
pub struct Settings(Arc<Inner>);

struct Inner {
    path: Option<PathBuf>,
    store: Arc<dyn SettingsStore>,
    current: RwLock<Arc<Config>>,
}

impl Settings {
    /// Fails when the stored settings do not load or validate.
    pub async fn open(path: Option<PathBuf>, store: Arc<dyn SettingsStore>) -> Result<Self> {
        let current = RwLock::new(Arc::new(read(path.as_ref(), store.as_ref()).await?));
        Ok(Self(Arc::new(Inner { path, store, current })))
    }

    pub fn current(&self) -> Arc<Config> {
        self.0.current.read().expect("settings lock").clone()
    }

    /// `read` applied to the configuration in effect each time the value is used.
    pub fn live<T>(&self, read: impl Fn(&Config) -> T + Send + Sync + 'static) -> Live<T> {
        let settings = self.clone();
        Live::new(move || read(&settings.current()))
    }

    /// Stores `value` for `key` over the config file and applies it; stores nothing when the
    /// configuration would not load or validate with it.
    pub async fn set(&self, key: &str, value: Value) -> Result<()> {
        Config::editable(key)?;
        let mut candidate = self.0.store.settings().await?;
        candidate.retain(|(stored, _)| stored != key);
        candidate.push((key.to_owned(), value.clone()));
        let config =
            Config::load(self.0.path.as_deref(), &candidate).with_context(|| format!("{key} cannot be {value}"))?;
        config.validate().with_context(|| format!("{key} cannot be {value}"))?;
        self.0.store.set_setting(key, &value).await?;
        self.reload().await
    }

    /// The configuration with `changes` over the stored settings, loaded and validated; nothing is
    /// stored or applied. A `None` value leaves its key to the config file.
    pub async fn preview(&self, changes: &[(String, Option<Value>)]) -> Result<Config> {
        let mut candidate = self.0.store.settings().await?;
        for (key, value) in changes {
            Config::editable(key)?;
            candidate.retain(|(stored, _)| stored != key);
            candidate.extend(value.clone().map(|value| (key.clone(), value)));
        }
        let config = Config::load(self.0.path.as_deref(), &candidate)?;
        config.validate()?;
        Ok(config)
    }

    /// Removes the stored value of `key` and applies the configuration without it; `false` when
    /// none was stored.
    pub async fn unset(&self, key: &str) -> Result<bool> {
        let removed = self.0.store.remove_setting(key).await?;
        self.reload().await?;
        Ok(removed)
    }

    /// The keys of the stored settings, in order.
    pub async fn stored_keys(&self) -> Result<Vec<String>> {
        Ok(self.0.store.settings().await?.into_iter().map(|(key, _)| key).collect())
    }

    /// Reads the config file and the stored settings again; on failure the configuration in effect
    /// stays.
    pub async fn reload(&self) -> Result<()> {
        let config = read(self.0.path.as_ref(), self.0.store.as_ref()).await?;
        *self.0.current.write().expect("settings lock") = Arc::new(config);
        info!("settings reloaded");
        Ok(())
    }
}

async fn read(path: Option<&PathBuf>, store: &dyn SettingsStore) -> Result<Config> {
    let config = Config::load(path.map(PathBuf::as_path), &store.settings().await?)?;
    config.validate()?;
    Ok(config)
}

#[async_trait]
impl Handler<SettingsChanged> for Settings {
    async fn handle(&self, _: &SettingsChanged) -> Result<(), HandlerError> {
        Ok(self.reload().await?)
    }
}
