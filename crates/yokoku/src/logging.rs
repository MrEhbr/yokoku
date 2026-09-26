use std::{fs::OpenOptions, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{Level, level_filters::LevelFilter};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

const QUIET_TARGETS: [&str; 9] = ["hyper", "h2", "tower", "reqwest::connect", "ureq", "rustls", "want", "mio", "tokio"];

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
/// Directives added to the configured level when `RUST_LOG` is unset.
const QUIET_DEPENDENCIES: &str =
    "hyper=warn,h2=warn,ureq=warn,rustls=warn,want=warn,mio=warn,tokio=warn,tokenizers=warn";

/// Filters by `RUST_LOG` when set, else by the configured level.
///
/// Returns a WorkerGuard that must be held until program exit to ensure log flush.
pub fn setup(config: &LogConfig) -> Result<WorkerGuard> {
    let level_filter: LevelFilter = config.level.into();
    let directives = match std::env::var(EnvFilter::DEFAULT_ENV) {
        Ok(directives) if !directives.is_empty() => directives,
        _ => format!("{level_filter},{QUIET_DEPENDENCIES}"),
    };
    let env_filter = EnvFilter::builder().parse_lossy(directives);

    let (writer, guard) = match &config.output {
        LogOutput::Stderr => tracing_appender::non_blocking(std::io::stderr()),
        LogOutput::Stdout => tracing_appender::non_blocking(std::io::stdout()),
        LogOutput::File(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .with_context(|| format!("Failed to open log file: {}", path.display()))?;
            tracing_appender::non_blocking(file)
        },
    };

    let ansi = !matches!(config.output, LogOutput::File(_));

    let fmt_layer = match config.format {
        LogFormat::Console => tracing_subscriber::fmt::layer()
            .compact()
            .with_line_number(true)
            .with_writer(writer)
            .with_ansi(ansi)
            .boxed(),
        LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(true)
            .with_writer(writer)
            .boxed(),
    }
    .with_filter(env_filter);

    let registry = tracing_subscriber::registry().with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    let registry = registry.with(console_subscriber::spawn());

    registry.try_init().context("Failed to initialize logging")?;

    Ok(guard)
}
