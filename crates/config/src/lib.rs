//! The configuration: every crate's settings, layered from defaults, the config file, stored
//! settings and the environment.

mod log;
mod sections;

use std::path::Path;

use anyhow::{Result, bail};
use config::{Environment, File, FileFormat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
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
};

const ENV_PREFIX: &str = "APP";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub clock: ClockSettings,
    #[serde(default)]
    pub metadata: MetadataSettings,
    #[serde(default)]
    pub transmission: TransmissionSettings,
    #[serde(default)]
    pub downloads: DownloadOptions,
    #[serde(default)]
    pub add: AddConfig,
    #[serde(default)]
    pub list: ListConfig,
    #[serde(default)]
    pub calendar: CalendarConfig,
    #[serde(default)]
    pub serve: ScheduleSettings,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub import: ImportSettings,
    #[serde(default)]
    pub jellyfin: JellyfinSettings,
    #[serde(default)]
    pub files: ProbeSettings,
    #[serde(default)]
    pub naming: Naming,
}

/// Load configuration with precedence: env vars (APP__*) > stored settings > config file > defaults.
/// `stored` holds values by dotted key, such as `import.mode`.
pub fn load<T>(config_path: Option<&Path>, stored: &[(String, Value)]) -> Result<T>
where
    T: DeserializeOwned + Serialize + Default,
{
    let mut builder = config::Config::builder().add_source(config::Config::try_from(&T::default())?);

    if let Some(path) = config_path {
        builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
    }
    if !stored.is_empty() {
        builder = builder.add_source(File::from_str(&nested(stored).to_string(), FileFormat::Json));
    }

    let config = builder.add_source(Environment::with_prefix(ENV_PREFIX).separator("__")).build()?;

    Ok(config.try_deserialize()?)
}

/// `a.b = 1` becomes `{ "a": { "b": 1 } }`.
fn nested(stored: &[(String, Value)]) -> Value {
    let mut root = Map::new();
    for (key, value) in stored {
        insert(&mut root, &key.split('.').collect::<Vec<_>>(), value.clone());
    }
    Value::Object(root)
}

fn insert(table: &mut Map<String, Value>, path: &[&str], value: Value) {
    match path {
        [] => {},
        [leaf] => {
            table.insert((*leaf).to_owned(), value);
        },
        [part, rest @ ..] => {
            let entry = table.entry(*part).or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            if let Value::Object(next) = entry {
                insert(next, rest, value);
            }
        },
    }
}

impl Config {
    /// Fails on a setting that would only fail later, when used.
    pub fn validate(&self) -> Result<()> {
        self.metadata.tvdb_language()?;
        self.serve.schedules()?;
        Ok(())
    }

    /// The value of a known setting, e.g. `import.mode`.
    pub fn setting(&self, key: &str) -> Result<Value> {
        let config = serde_json::to_value(self)?;
        match key.split('.').try_fold(&config, |value, part| value.get(part)) {
            Some(value) if !value.is_object() => Ok(value.clone()),
            _ => bail!("{key} is not a setting"),
        }
    }

    /// `value` as `setting` shows it once loaded, so a secret reads `"<redacted>"`; a value that does
    /// not load is shown as stored.
    pub fn shown(key: &str, value: &Value) -> Value {
        match load::<Self>(None, &[(key.to_owned(), value.clone())]).and_then(|config| config.setting(key)) {
            Ok(loaded) if loaded == REDACTED => loaded,
            _ => value.clone(),
        }
    }

    /// Fails unless `key` is a setting the database can store.
    pub fn editable(key: &str) -> Result<()> {
        Self::default().setting(key)?;
        if ["database", "log"].contains(&key.split('.').next().unwrap_or_default()) {
            bail!("{key} is needed before the database opens; set it in the config file or environment");
        }
        Ok(())
    }
}
