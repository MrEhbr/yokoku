use std::path::Path;

use anyhow::Result;
use config::{Environment, File, FileFormat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{commands::greet::GreetConfig, logging::LogConfig};

const ENV_PREFIX: &str = "APP";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub greet: GreetConfig,
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
