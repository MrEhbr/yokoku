//! The configuration: every crate's settings, layered from defaults, the config file, stored
//! settings and the environment.

mod log;
mod sections;
mod settings;

use std::path::Path;

use anyhow::{Result, bail};
use config::{Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use yokoku_domain::REDACTED;
use yokoku_downloads::DownloadOptions;
use yokoku_jobs::ScheduleSettings;
use yokoku_media::ImportSettings;
use yokoku_metadata::MetadataSettings;
use yokoku_naming::Naming;
use yokoku_system::{ClockSettings, JellyfinSettings, ProbeSettings};
use yokoku_transmission::TransmissionSettings;

pub use crate::{
    log::{LogConfig, LogFormat, LogOutput},
    sections::{AddConfig, CalendarConfig, DatabaseConfig, ListConfig, WebConfig},
    settings::Settings,
};

const ENV_PREFIX: &str = "APP";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Config {
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub clock: ClockSettings,
    pub metadata: MetadataSettings,
    pub transmission: TransmissionSettings,
    pub downloads: DownloadOptions,
    pub add: AddConfig,
    pub list: ListConfig,
    pub calendar: CalendarConfig,
    pub serve: ScheduleSettings,
    pub web: WebConfig,
    pub import: ImportSettings,
    pub jellyfin: JellyfinSettings,
    pub files: ProbeSettings,
    pub naming: Naming,
}

impl Config {
    /// Loads with precedence: env vars (APP__*) > stored settings > config file > defaults.
    /// `stored` holds values by dotted key, such as `import.mode`.
    pub fn load(config_path: Option<&Path>, stored: &[(String, Value)]) -> Result<Self> {
        let mut builder = config::Config::builder().add_source(config::Config::try_from(&Self::default())?);

        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
        }
        let mut stored_layer = config::Config::builder();
        for (key, value) in stored {
            stored_layer = stored_layer.set_override(key, config::Value::deserialize(value)?)?;
        }
        builder = builder.add_source(stored_layer.build()?);

        let config = builder.add_source(Environment::with_prefix(ENV_PREFIX).separator("__")).build()?;

        Ok(config.try_deserialize()?)
    }

    /// Fails on a setting that would only fail later, when used.
    pub fn validate(&self) -> Result<()> {
        self.metadata.tvdb_language()?;
        self.serve.schedules()?;
        Ok(())
    }

    /// The value of a known setting, e.g. `import.mode`, as JSON: `"copy"`, `14`.
    pub fn setting(&self, key: &str) -> Result<String> {
        Ok(self.value(key)?.to_string())
    }

    /// `value` as `setting` shows it once loaded, so a secret reads `"<redacted>"`; a value that does
    /// not load is shown as stored.
    pub fn shown(key: &str, value: &Value) -> String {
        match Self::load(None, &[(key.to_owned(), value.clone())]).and_then(|config| config.value(key)) {
            Ok(loaded) if loaded == REDACTED => loaded.to_string(),
            _ => value.to_string(),
        }
    }

    /// Fails unless `key` is a setting the database can store.
    pub fn editable(key: &str) -> Result<()> {
        Self::default().value(key)?;
        if ["database", "log"].contains(&key.split('.').next().unwrap_or_default()) {
            bail!("{key} is needed before the database opens; set it in the config file or environment");
        }
        Ok(())
    }

    fn value(&self, key: &str) -> Result<Value> {
        let config = serde_json::to_value(self)?;
        match key.split('.').try_fold(&config, |value, part| value.get(part)) {
            Some(value) if !value.is_object() => Ok(value.clone()),
            _ => bail!("{key} is not a setting"),
        }
    }
}
