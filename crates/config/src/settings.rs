use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;
use yokoku_domain::{Live, SettingsStore};
use yokoku_events::{Handler, HandlerError, SettingsChanged};

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
