use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::Level;

fn serialize_level<S>(level: &Option<Level>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match level {
        Some(l) => serializer.serialize_str(&l.to_string().to_lowercase()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<Option<Level>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => Ok(Some(s.parse::<Level>().map_err(serde::de::Error::custom)?)),
        None => Ok(None),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Console,
    Json,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    #[default]
    Stderr,
    Stdout,
    File(PathBuf),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    #[serde(serialize_with = "serialize_level", deserialize_with = "deserialize_level")]
    pub level: Option<Level>,
    #[serde(default)]
    pub format: LogFormat,
    #[serde(default)]
    pub output: LogOutput,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self { level: Some(Level::INFO), format: LogFormat::Console, output: LogOutput::Stderr }
    }
}
