use std::path::Path;

use anyhow::Result;
use config::{Environment, File, FileFormat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    app::{ClockConfig, DatabaseConfig, MetadataConfig, TransmissionConfig},
    commands::{
        add::AddConfig, greet::GreetConfig, import::ImportConfig, jellyfin::JellyfinConfig, list::ListConfig,
        recycle::RecycleConfig, serve::ServeConfig, upcoming::UpcomingConfig,
    },
    logging::LogConfig,
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
    pub greet: GreetConfig,
    #[serde(default)]
    pub add: AddConfig,
    #[serde(default)]
    pub list: ListConfig,
    #[serde(default)]
    pub upcoming: UpcomingConfig,
    #[serde(default)]
    pub serve: ServeConfig,
    #[serde(default)]
    pub import: ImportConfig,
    #[serde(default)]
    pub recycle: RecycleConfig,
    #[serde(default)]
    pub jellyfin: JellyfinConfig,
}

/// Load configuration with precedence: env vars (APP__*) > config file > defaults.
pub fn load<T>(config_path: Option<&Path>) -> Result<T>
where
    T: DeserializeOwned + Serialize + Default,
{
    let mut builder = config::Config::builder().add_source(config::Config::try_from(&T::default())?);

    if let Some(path) = config_path {
        builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
    }

    let config = builder.add_source(Environment::with_prefix(ENV_PREFIX).separator("__")).build()?;

    Ok(config.try_deserialize()?)
}
