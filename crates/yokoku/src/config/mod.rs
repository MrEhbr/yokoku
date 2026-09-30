//! The configuration: every crate's settings, layered from defaults, the config file, stored
//! settings and the environment.

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
    sections::{AddConfig, CalendarConfig, DatabaseConfig, EventsConfig, ListConfig, WebConfig},
    settings::Settings,
};
use crate::jobs::ScheduleSettings;

const ENV_PREFIX: &str = "APP";
const ENV_SEPARATOR: &str = "__";

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
    pub list: ListConfig,
    pub calendar: CalendarConfig,
    pub serve: ScheduleSettings,
    pub events: EventsConfig,
    pub import: ImportSettings,
    pub jellyfin: JellyfinSettings,
    pub files: ProbeSettings,
    pub naming: Naming,
}

impl Config {
    /// Loads with precedence: env vars (APP__*) > stored settings > config file > defaults.
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

    /// A stored value as `setting` shows it over the defaults alone, so a secret is masked; as stored
    /// when it does not load.
    pub fn shown(key: &str, stored: &Value) -> String {
        let alone = [(key.to_owned(), stored.clone())];
        let loaded = Self::layers(None, &alone).and_then(|layers| Ok(layers.build()?.try_deserialize::<Self>()?));
        loaded.and_then(|config| config.setting(key)).unwrap_or_else(|_| stored.to_string())
    }

    /// An `APP__` environment variable, or its `__FILE` form, sets `key` over any stored value.
    pub fn set_by_env(key: &str) -> bool {
        let variable = format!("{ENV_PREFIX}{ENV_SEPARATOR}{}", key.to_uppercase().replace('.', ENV_SEPARATOR));
        env::var_os(&variable).is_some() || env::var_os(format!("{variable}{ENV_SEPARATOR}FILE")).is_some()
    }

    /// Fails unless `key` is a setting the database can store.
    pub fn editable(key: &str) -> Result<()> {
        Self::default().value(key)?;
        if ["database", "log"].contains(&key.split('.').next().unwrap_or_default()) {
            bail!("{key} is needed before the database opens; set it in the config file or environment");
        }
        Ok(())
    }

    /// The value of a known setting as JSON, a secret masked.
    pub fn value(&self, key: &str) -> Result<Value> {
        let config = Secret::masking(|| serde_json::to_value(self))?;
        match key.split('.').try_fold(&config, |value, part| value.get(part)) {
            Some(value) if !value.is_object() => Ok(value.clone()),
            _ => bail!("{key} is not a setting"),
        }
    }
}

#[cfg(test)]
mod tests;
