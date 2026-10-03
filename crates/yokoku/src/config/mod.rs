//! The configuration: every crate's settings, layered from defaults, the config file, stored
//! settings and the environment.

mod fields;
mod log;
mod sections;
mod settings;
mod web;

use std::{env, path::Path};

use anyhow::{Result, bail};
use config::{ConfigBuilder, Environment, File, FileFormat, builder::DefaultState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use yokoku_core::{downloads::DownloadOptions, media::ImportSettings};
use yokoku_domain::{Secret, naming::Naming};
use yokoku_infra::{
    download_clients::TransmissionSettings,
    media_servers::JellyfinSettings,
    metadata::MetadataSettings,
    system::{ClockSettings, ProbeSettings},
};

pub use crate::config::{
    log::{LogConfig, LogFormat, LogOutput},
    sections::{AddConfig, DatabaseConfig, EventsConfig, RootConfig, WebConfig},
    settings::Settings,
};
use crate::jobs::ScheduleSettings;

const ENV_PREFIX: &str = "YOKOKU";
const ENV_SEPARATOR: &str = "__";
/// Sections read once, when the service starts.
const READ_AT_START: [&str; 5] = ["database", "log", "web", "events", "roots"];

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Config {
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub web: WebConfig,
    pub clock: ClockSettings,
    pub metadata: MetadataSettings,
    pub transmission: TransmissionSettings,
    pub downloads: DownloadOptions,
    pub add: AddConfig,
    pub serve: ScheduleSettings,
    pub events: EventsConfig,
    pub import: ImportSettings,
    pub jellyfin: JellyfinSettings,
    pub files: ProbeSettings,
    pub naming: Naming,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roots: Vec<RootConfig>,
}

impl Config {
    /// Loads with precedence: env vars (YOKOKU__*) > stored settings > config file > defaults.
    /// `stored` holds values by dotted key, such as `import.mode`.
    pub fn load(config_path: Option<&Path>, stored: &[(String, Value)]) -> Result<Self> {
        let layers = Self::layers(config_path, stored)?;
        Ok(layers
            .add_source(Environment::with_prefix(ENV_PREFIX).separator(ENV_SEPARATOR))
            .build()?
            .try_deserialize()?)
    }

    /// Defaults, the config file, then `stored`.
    fn layers(config_path: Option<&Path>, stored: &[(String, Value)]) -> Result<ConfigBuilder<DefaultState>> {
        let mut builder = config::Config::builder().add_source(config::Config::try_from(&Self::default())?);

        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
        }
        let mut stored_layer = config::Config::builder();
        for (key, value) in stored {
            stored_layer = stored_layer.set_override(key, config::Value::deserialize(value)?)?;
        }
        Ok(builder.add_source(stored_layer.build()?))
    }

    /// Fails on a setting that would only fail later, when used.
    pub fn validate(&self) -> Result<()> {
        self.metadata.tvdb_language()?;
        Ok(())
    }

    /// The value of a known setting, e.g. `import.mode`, as JSON: `"copy"`, `14`; a secret masked.
    pub fn setting(&self, key: &str) -> Result<String> {
        Ok(self.value(key)?.to_string())
    }

    /// A stored value as `setting` shows it over the defaults alone, a secret masked; as stored when
    /// it does not load.
    pub fn shown(key: &str, stored: &Value) -> String {
        let alone = [(key.to_owned(), stored.clone())];
        let loaded = Self::layers(None, &alone).and_then(|layers| Ok(layers.build()?.try_deserialize::<Self>()?));
        loaded.and_then(|config| config.setting(key)).unwrap_or_else(|_| stored.to_string())
    }

    /// A `YOKOKU__` environment variable, or its `__FILE` form, sets `key` over any stored value.
    pub fn set_by_env(key: &str) -> bool {
        let variable = format!("{ENV_PREFIX}{ENV_SEPARATOR}{}", key.to_uppercase().replace('.', ENV_SEPARATOR));
        env::var_os(&variable).is_some() || env::var_os(format!("{variable}{ENV_SEPARATOR}FILE")).is_some()
    }

    /// Fails unless `key` is a setting the database can store.
    pub fn editable(key: &str) -> Result<()> {
        Self::default().value(key)?;
        if READ_AT_START.contains(&key.split('.').next().unwrap_or_default()) {
            bail!("{key} is read when the service starts; set it in the config file or environment and restart");
        }
        Ok(())
    }

    /// The value of a known setting as JSON, a secret masked.
    pub fn value(&self, key: &str) -> Result<Value> {
        match Self::default().lookup(key)? {
            Some(default) if !default.is_object() => Ok(self.lookup(key)?.unwrap_or(Value::Null)),
            _ => bail!("{key} is not a setting"),
        }
    }

    /// The JSON at dotted `key`, a secret masked; an object for a section.
    fn lookup(&self, key: &str) -> Result<Option<Value>> {
        let config = Secret::masking(|| serde_json::to_value(self))?;
        Ok(key.split('.').try_fold(&config, |value, part| value.get(part)).cloned())
    }
}

#[cfg(test)]
mod tests;
