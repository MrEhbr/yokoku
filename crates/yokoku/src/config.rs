use std::path::Path;

use anyhow::Result;
use config::{Environment, File, FileFormat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{
    app::{ClockConfig, DatabaseConfig, MetadataConfig, NamingConfig, TransmissionConfig},
    cli::commands::{
        add::AddConfig, calendar::CalendarConfig, files::FilesConfig, import::ImportConfig, jellyfin::JellyfinConfig,
        list::ListConfig,
    },
    logging::LogConfig,
    service::{ServeConfig, WebConfig},
};

const ENV_PREFIX: &str = "APP";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub clock: ClockConfig,
    #[serde(default)]
    pub metadata: MetadataConfig,
    #[serde(default)]
    pub transmission: TransmissionConfig,
    #[serde(default)]
    pub add: AddConfig,
    #[serde(default)]
    pub list: ListConfig,
    #[serde(default)]
    pub calendar: CalendarConfig,
    #[serde(default)]
    pub serve: ServeConfig,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub import: ImportConfig,
    #[serde(default)]
    pub jellyfin: JellyfinConfig,
    #[serde(default)]
    pub files: FilesConfig,
    #[serde(default)]
    pub naming: NamingConfig,
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
        self.naming.naming()?;
        self.clock.time_zone()?;
        self.metadata.tvdb_language()?;
        self.serve.schedules()?;
        Ok(())
    }
}
