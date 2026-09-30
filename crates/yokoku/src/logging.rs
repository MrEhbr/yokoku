use std::fs::OpenOptions;

use anyhow::{Context, Result};
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{LogConfig, LogFormat, LogOutput};

/// Directives added to the configured level when `RUST_LOG` is unset.
const QUIET_DEPENDENCIES: &str =
    "hyper=warn,h2=warn,rustls=warn,want=warn,mio=warn,tokio=warn,reqwest=warn,sqlx=warn,apalis=warn";

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
